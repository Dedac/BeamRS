#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(git -C "$script_dir" rev-parse --show-toplevel)
crate_dir="$repo_root/beamrs"
compose_file="$crate_dir/docker-compose.growth.yml"
project_name="beamrs-growth-$(date +%s)-$$"

ray_count=${RAY_COUNT:-100000}
feed_page_size=${FEED_PAGE_SIZE:-100}
concurrency=${CONCURRENCY:-8}
requests=${REQUESTS:-300}
report_path=${REPORT_PATH:-target/growth-report-$(date -u +%Y%m%dT%H%M%SZ).md}

fail() {
    printf '%s\n' "$*" >&2
    exit 2
}

validate_integer() {
    name=$1
    value=$2
    minimum=$3
    maximum=$4
    case "$value" in
        ''|*[!0-9]*) fail "$name must be an integer between $minimum and $maximum" ;;
    esac
    if [ "${#value}" -gt 9 ] || [ "$value" -lt "$minimum" ] || [ "$value" -gt "$maximum" ]; then
        fail "$name must be an integer between $minimum and $maximum"
    fi
}

command -v docker >/dev/null 2>&1 || fail "docker is required"
command -v cargo >/dev/null 2>&1 || fail "cargo is required"
docker compose version >/dev/null 2>&1 || fail "Docker Compose is required"
docker info >/dev/null 2>&1 || fail "the Docker daemon is not available"

validate_integer RAY_COUNT "$ray_count" 1 1000000
validate_integer FEED_PAGE_SIZE "$feed_page_size" 1 100
validate_integer CONCURRENCY "$concurrency" 1 64
validate_integer REQUESTS "$requests" 1 100000

cd "$crate_dir"
if [ -e "$report_path" ]; then
    fail "report already exists: $report_path"
fi

compose() {
    docker compose -p "$project_name" -f "$compose_file" "$@"
}

cleanup() {
    result=$?
    trap - EXIT
    if ! compose down --remove-orphans; then
        printf '%s\n' "failed to remove disposable growth-test containers" >&2
        if [ "$result" -eq 0 ]; then
            result=1
        fi
    fi
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

compose up -d --wait db
host_port=$(compose port db 5432 | sed 's/.*://')
database_url="postgres://beamrs:beamrs@127.0.0.1:$host_port/beamrs"

# The first run applies migrations and seeds the standard demo rows.
DATABASE_URL="$database_url" cargo run --release --quiet --bin loadtest -- \
    --scenario health --requests 1 --concurrency 1 >/dev/null

compose exec -T db psql -X -U beamrs -d beamrs \
    --set=ON_ERROR_STOP=1 --set=ray_count="$ray_count" <<'SQL'
INSERT INTO frequencies (name)
VALUES ('growth-benchmark')
ON CONFLICT (name) DO NOTHING;

INSERT INTO users (username)
SELECT 'GrowthUser' || number
FROM generate_series(1, 100) AS series(number)
ON CONFLICT (username) DO NOTHING;

WITH frequency AS (
    SELECT id FROM frequencies WHERE name = 'growth-benchmark'
),
growth_users AS (
    SELECT id, username FROM users WHERE username LIKE 'GrowthUser%'
)
INSERT INTO rays (frequency_id, user_id, text, created_at)
SELECT
    frequency.id,
    growth_user.id,
    'Growth ray ' || series.number,
    TIMESTAMPTZ '2024-01-01 00:00:00+00'
        + (series.number - 1) * INTERVAL '1 second'
FROM generate_series(1, :ray_count) AS series(number)
JOIN growth_users AS growth_user
    ON growth_user.username = 'GrowthUser' || (((series.number - 1) % 100) + 1)
CROSS JOIN frequency;

INSERT INTO prisms (user_id, ray_id)
SELECT growth_user.id, ray.id
FROM rays AS ray
JOIN frequencies AS frequency ON frequency.id = ray.frequency_id
CROSS JOIN users AS growth_user
WHERE frequency.name = 'growth-benchmark'
  AND growth_user.username = 'GrowthUser1'
  AND ray.text LIKE 'Growth ray %'
  AND split_part(ray.text, ' ', 3)::BIGINT % 10 = 0
ON CONFLICT (user_id, ray_id) DO NOTHING;

ANALYZE rays;
ANALYZE prisms;
SQL

frequency_id=$(compose exec -T db psql -X -At -U beamrs -d beamrs \
    --set=ON_ERROR_STOP=1 \
    --command="SELECT id FROM frequencies WHERE name = 'growth-benchmark'")
postgres_version=$(compose exec -T db psql -X -At -U beamrs -d beamrs \
    --command="SELECT version()")
database_bytes=$(compose exec -T db psql -X -At -U beamrs -d beamrs \
    --command="SELECT pg_database_size('beamrs')")
rust_version=$(rustc --version)
cargo_version=$(cargo --version)
compose_version=$(docker compose version --short)
platform=$(uname -srm)

DATABASE_URL="$database_url" cargo run --release --quiet --bin loadtest -- \
    --scenario read-mix --frequency-id "$frequency_id" --username GrowthUser1 \
    --feed-page-size "$feed_page_size" --concurrency 1 --requests 20 >/dev/null
load_report=$(DATABASE_URL="$database_url" cargo run --release --quiet --bin loadtest -- \
    --scenario read-mix --frequency-id "$frequency_id" --username GrowthUser1 \
    --feed-page-size "$feed_page_size" --concurrency "$concurrency" \
    --requests "$requests" --json)

# Mirrors BeamRepository::list_rays_where in src/repository.rs.
feed_sql() {
    predicate=$1
    cursor_clause=$2
    cat <<EOF
WITH page AS (
    SELECT r.id, r.text, r.frequency_id, r.user_id, r.created_at
    FROM rays r
    WHERE $predicate
      $cursor_clause
    ORDER BY r.created_at DESC, r.id DESC
    LIMIT \$2
)
SELECT page.id, page.text, page.frequency_id, page.user_id,
       COALESCE((SELECT author.username FROM users author
                 WHERE author.id = page.user_id), '') AS user_name,
       prism_stats.prism_count,
       FLOOR(EXTRACT(EPOCH FROM page.created_at) * 1000000)::BIGINT
           AS cursor_created_at_micros,
       prism_stats.users_prismed
FROM page
CROSS JOIN LATERAL (
    SELECT COUNT(*)::INT AS prism_count,
           COALESCE(ARRAY_AGG(pu.username ORDER BY pu.username)
               FILTER (WHERE pu.username IS NOT NULL), ARRAY[]::TEXT[]) AS users_prismed
    FROM prisms p
    LEFT JOIN users pu ON pu.id = p.user_id
    WHERE p.ray_id = page.id
) prism_stats
ORDER BY page.created_at DESC, page.id DESC
EOF
}

cursor_clause="AND (r.created_at, r.id) <
          (TIMESTAMPTZ 'epoch' + (\$3::DOUBLE PRECISION * INTERVAL '1 microsecond'),
           \$4::INTEGER)"

# SQLx reuses prepared statements, so forced generic plans show whether each
# feed stays index-bounded for any parameter values. Generic plans cannot see
# LIMIT $2 and estimate a large page; on big feeds that can cross PostgreSQL's
# jit_above_cost, so first-page timings may include one-time JIT compilation.
# The deep page starts halfway down each feed.
explain_feed() {
    label=$1
    name=$2
    param_type=$3
    param_value=$4
    predicate=$5
    cat <<EOF
\\echo '$label: first page'
PREPARE ${name}_first($param_type, BIGINT) AS
$(feed_sql "$predicate" "");
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) EXECUTE ${name}_first($param_value, :page_limit);
PREPARE ${name}_cursor($param_type) AS
SELECT FLOOR(EXTRACT(EPOCH FROM r.created_at) * 1000000)::BIGINT AS cursor_micros,
       r.id AS cursor_id
FROM rays r
WHERE $predicate
ORDER BY r.created_at DESC, r.id DESC
OFFSET (SELECT COUNT(*) / 2 FROM rays r WHERE $predicate)
LIMIT 1;
EXECUTE ${name}_cursor($param_value) \\gset
\\echo '$label: page starting at the feed midpoint'
PREPARE ${name}_deep($param_type, BIGINT, BIGINT, INTEGER) AS
$(feed_sql "$predicate" "$cursor_clause");
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
EXECUTE ${name}_deep($param_value, :page_limit, :cursor_micros, :cursor_id);
EOF
}

query_plans=$(
    {
        printf '%s\n' 'SET plan_cache_mode = force_generic_plan;'
        explain_feed 'Frequency feed' frequency_feed INTEGER :frequency_id \
            'r.frequency_id = $1'
        explain_feed 'Authored feed' authored_feed TEXT "'GrowthUser1'" \
            'r.user_id = (SELECT id FROM users WHERE username = $1)'
        explain_feed 'Prismed feed' prismed_feed TEXT "'GrowthUser1'" \
            'EXISTS (
                SELECT 1
                FROM prisms selected_prism
                JOIN users selected_user ON selected_user.id = selected_prism.user_id
                WHERE selected_prism.ray_id = r.id AND selected_user.username = $1
            )'
        # PostgreSQL chooses between custom and generic plans after five
        # executions; record the plan it actually keeps by default.
        cat <<EOF
SET plan_cache_mode = auto;
\\echo 'Frequency feed: first page under default plan_cache_mode after 6 executions'
PREPARE frequency_feed_auto(INTEGER, BIGINT) AS
$(feed_sql 'r.frequency_id = $1' "");
\\o /dev/null
EXECUTE frequency_feed_auto(:frequency_id, :page_limit);
EXECUTE frequency_feed_auto(:frequency_id, :page_limit);
EXECUTE frequency_feed_auto(:frequency_id, :page_limit);
EXECUTE frequency_feed_auto(:frequency_id, :page_limit);
EXECUTE frequency_feed_auto(:frequency_id, :page_limit);
EXECUTE frequency_feed_auto(:frequency_id, :page_limit);
\\o
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
EXECUTE frequency_feed_auto(:frequency_id, :page_limit);
EOF
    } | compose exec -T db psql -X -U beamrs -d beamrs \
        --set=ON_ERROR_STOP=1 \
        --set=frequency_id="$frequency_id" \
        --set=page_limit="$((feed_page_size + 1))"
)

working_tree_state=clean
if [ -n "$(git -C "$repo_root" status --porcelain)" ]; then
    working_tree_state=dirty
fi
mkdir -p "$(dirname "$report_path")"
cat >"$report_path" <<EOF
# BeamRS growth benchmark

- Generated: $(date -u '+%Y-%m-%dT%H:%M:%SZ')
- Source commit: $(git -C "$repo_root" rev-parse HEAD)
- Worktree state before report creation: $working_tree_state
- Platform: $platform
- Rust: $rust_version
- Cargo: $cargo_version
- Docker Compose: $compose_version
- PostgreSQL: $postgres_version
- Dataset: $ray_count rays, 100 synthetic users, 10% prismed by GrowthUser1
- Feed page size: $feed_page_size
- Load: $requests requests at concurrency $concurrency, after a 20-request warm-up
- Database size: $database_bytes bytes
- Scope: isolated disposable PostgreSQL container; not a production SLO claim

## Load report

\`\`\`json
$load_report
\`\`\`

## Query plans

\`\`\`text
$query_plans
\`\`\`
EOF

printf 'Growth report written to %s\n' "$report_path"
