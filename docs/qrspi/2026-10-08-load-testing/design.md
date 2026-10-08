---
status: approved
phase: design
---

# Design

Add an in-process load generator that drives the application's own
`axum::Router` through `tower::ServiceExt::oneshot`.

## Components

- `beamrs::loadtest` (`ssr` only): the engine and its value types.
  - `RequestSpec` — a named, weighted request: method, path, optional JSON
    body.
  - `LoadProfile` — `concurrency`, `total_requests`, and the `RequestSpec`
    mix, with a `validate()` that rejects unusable profiles up front.
  - `run_load_test(router, profile)` — spawns `concurrency` worker tasks that
    pull request slots from a shared atomic counter until `total_requests` is
    exhausted, times each request including body drain, and aggregates the
    samples.
  - `LoadReport` / `LatencyStats` — serializable results with overall and
    per-endpoint volume, failures, status histogram, throughput, and
    nearest-rank percentiles; `render_text()` for operators.
- `beamrs` binary `loadtest`: parses flags, loads `Settings`, connects to
  PostgreSQL, builds the real router, runs a named scenario, prints a text or
  JSON report, and exits non-zero when any request fails.

## Key decisions

- **In-process, not over a socket.** It reuses the lockfile's dependencies,
  needs no running server, and isolates application latency. The excluded
  network layer is documented.
- **Deterministic weighted schedule.** Weights expand into a schedule vector
  indexed by the global request number, so the endpoint mix is reproducible
  and testable rather than randomly sampled.
- **Failure definition.** A sample fails when its status is >= 400 or the
  request could not be built. Failures still contribute latency samples, and
  the status histogram preserves the detail.
- **No behavior change.** The module is additive; `server.rs`, `app.rs`,
  `repository.rs`, and the schema are untouched.

## Per-goal acceptance

| Goal | Verified by |
|---|---|
| Configurable concurrency, volume, mix | `LoadProfile` fields + engine tests |
| Full report content | `LoadReport` tests over a known router |
| Invalid profiles rejected | `validate()` tests for each failure mode |
| Operator entry point + docs | `loadtest` binary + `README.md` section |
| Validation passes | fmt, clippy, `cargo test --all-targets`, validator |
