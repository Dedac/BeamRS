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

query_plans=$(compose exec -T db psql -X -U beamrs -d beamrs \
    --set=ON_ERROR_STOP=1 \
    --set=frequency_id="$frequency_id" \
    --set=feed_page_size="$feed_page_size" <<'SQL'
\echo 'Frequency feed'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT r.id, r.text, r.frequency_id, r.user_id,
       COALESCE(u.username, '') AS user_name,
       COUNT(p.id)::INT AS prism_count,
       COALESCE(ARRAY_AGG(pu.username ORDER BY pu.username)
           FILTER (WHERE pu.username IS NOT NULL), ARRAY[]::TEXT[]) AS users_prismed
FROM rays r
LEFT JOIN users u ON u.id = r.user_id
LEFT JOIN prisms p ON p.ray_id = r.id
LEFT JOIN users pu ON pu.id = p.user_id
WHERE r.frequency_id = :frequency_id
  AND (NULL::BIGINT IS NULL OR (r.created_at, r.id) < (TIMESTAMPTZ 'epoch', 0))
GROUP BY r.id, u.username
ORDER BY r.created_at DESC, r.id DESC
LIMIT :feed_page_size;

\echo 'Authored feed'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT r.id, r.text, r.frequency_id, r.user_id,
       COALESCE(u.username, '') AS user_name,
       COUNT(p.id)::INT AS prism_count,
       COALESCE(ARRAY_AGG(pu.username ORDER BY pu.username)
           FILTER (WHERE pu.username IS NOT NULL), ARRAY[]::TEXT[]) AS users_prismed
FROM rays r
LEFT JOIN users u ON u.id = r.user_id
LEFT JOIN prisms p ON p.ray_id = r.id
LEFT JOIN users pu ON pu.id = p.user_id
WHERE u.username = 'GrowthUser1'
  AND (NULL::BIGINT IS NULL OR (r.created_at, r.id) < (TIMESTAMPTZ 'epoch', 0))
GROUP BY r.id, u.username
ORDER BY r.created_at DESC, r.id DESC
LIMIT :feed_page_size;

\echo 'Prismed feed'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT r.id, r.text, r.frequency_id, r.user_id,
       COALESCE(u.username, '') AS user_name,
       COUNT(p.id)::INT AS prism_count,
       COALESCE(ARRAY_AGG(pu.username ORDER BY pu.username)
           FILTER (WHERE pu.username IS NOT NULL), ARRAY[]::TEXT[]) AS users_prismed
FROM rays r
LEFT JOIN users u ON u.id = r.user_id
LEFT JOIN prisms p ON p.ray_id = r.id
LEFT JOIN users pu ON pu.id = p.user_id
WHERE EXISTS (
    SELECT 1
    FROM prisms selected_prism
    JOIN users selected_user ON selected_user.id = selected_prism.user_id
    WHERE selected_prism.ray_id = r.id
      AND selected_user.username = 'GrowthUser1'
)
  AND (NULL::BIGINT IS NULL OR (r.created_at, r.id) < (TIMESTAMPTZ 'epoch', 0))
GROUP BY r.id, u.username
ORDER BY r.created_at DESC, r.id DESC
LIMIT :feed_page_size;
SQL
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
