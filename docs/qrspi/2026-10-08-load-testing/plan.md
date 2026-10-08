---
status: approved
phase: plan
phase_start_commit: 5de76cbf0caadce54f7273ccc4f509d15948ea5d
---

# Plan

## Task 1 — Load-testing engine (`tasks/task-01-engine.md`)

Add `beamrs/src/loadtest.rs` and declare it in `lib.rs`.

Test expectations:

- A profile with a single endpoint and N requests produces a report with
  `total_requests == N` and matching per-endpoint counts.
- A mixed profile honors weights deterministically.
- 4xx/5xx responses are counted as failures and appear in the status
  histogram while still contributing latency samples.
- `validate()` rejects zero concurrency, zero requests, an empty mix, an
  all-zero-weight mix, and a malformed path.
- Percentiles are nearest-rank and monotonic (p50 <= p95 <= p99 <= max).
- Concurrency above one still completes exactly `total_requests` requests.

## Task 2 — Operator binary (`tasks/task-02-cli.md`)

Add `beamrs/src/bin/loadtest.rs` plus the `Cargo.toml` target and
`bin-target` metadata.

Test expectations:

- Argument parsing is a pure function with unit tests: defaults, every flag,
  unknown flag, missing value, non-numeric value, unknown scenario.
- Scenario construction yields profiles that pass `validate()`.
- Scenarios perform no unbounded writes (user upsert is idempotent).

## Task 3 — Documentation (`tasks/task-03-docs.md`)

Add the `README.md` load-testing section: prerequisites, commands, flags,
scenarios, sample output, exit status, and the in-process caveat.

## Task 4 — Validation (`tasks/task-04-validation.md`)

Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --features ssr
-- -D warnings`, `cargo test --all-targets`, and
`./scripts/validate-qrspi.sh docs/qrspi/2026-10-08-load-testing`; record exact
results in `reviews/test-review.md`.
