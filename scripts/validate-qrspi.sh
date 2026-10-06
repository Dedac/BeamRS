#!/bin/sh
set -eu

usage() {
    printf '%s\n' "Usage: $0 RUN_DIRECTORY" >&2
    exit 2
}

[ "$#" -eq 1 ] || usage
run_dir=$1

[ -d "$run_dir" ] || {
    printf 'error: run directory does not exist: %s\n' "$run_dir" >&2
    exit 1
}

required="config.md goals.md questions.md design.md phasing.md roadmap.md structure.md plan.md parallelization.md"
missing=0

for file in $required; do
    if [ ! -f "$run_dir/$file" ]; then
        printf 'missing: %s\n' "$run_dir/$file" >&2
        missing=1
    fi
done

if [ ! -d "$run_dir/research" ]; then
    printf 'missing: %s/research/\n' "$run_dir" >&2
    missing=1
fi

if [ ! -d "$run_dir/tasks" ]; then
    printf 'missing: %s/tasks/\n' "$run_dir" >&2
    missing=1
fi

if [ "$missing" -ne 0 ]; then
    exit 1
fi

for file in $required; do
    if ! grep -q '^status:' "$run_dir/$file"; then
        printf 'error: %s has no status frontmatter\n' "$run_dir/$file" >&2
        missing=1
    fi
done

if [ "$missing" -ne 0 ]; then
    exit 1
fi

printf 'QRSPI run structure is valid: %s\n' "$run_dir"

