---
status: complete
phase: implement
task: task-01-engine
---

# Task 1 — Load-testing engine

## Scope

`beamrs/src/loadtest.rs` plus the `lib.rs` declaration.

## Outcome

`RequestSpec`, `LoadProfile`, `LatencyStats`, `LoadReport`, `LoadTestError`,
and `run_load_test` are implemented. Workers claim request slots from a shared
`AtomicUsize`, so exactly `total_requests` requests are sent regardless of how
unevenly they complete. Response bodies are drained with `axum::body::to_bytes`
before the sample is timed. Percentiles are nearest-rank over the full sorted
sample vector.

## Tests

13 unit tests in the module cover totals, deterministic weighting, zero-weight
exclusion, failure accounting with latency retention, JSON bodies, concurrency
above one, each `validate()` rejection, nearest-rank percentiles, percentile
monotonicity, and text rendering. They run against a throwaway `axum::Router`
and need no database.
