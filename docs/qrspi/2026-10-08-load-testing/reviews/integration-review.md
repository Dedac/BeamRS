---
status: complete
phase: integrate
---

# Integration review

## Diff scope

Added: `beamrs/src/loadtest.rs`, `beamrs/src/bin/loadtest.rs`,
`docs/qrspi/2026-10-08-load-testing/`.
Modified: `beamrs/src/lib.rs` (one module declaration), `beamrs/Cargo.toml`
(binary targets and `bin-target`), `README.md` (documentation).

## Checks

- No existing route, handler, extractor, domain rule, migration, or UI
  component was changed. The existing 29 tests still pass unchanged.
- Production dependencies do not change. `futures-util` is a dev-only
  dependency used to test a failing streaming response body.
- `loadtest` imports only `axum`, `tower`, `tokio`, `serde`, and `thiserror`.
  Nothing in the request path imports `loadtest`.
- The module and binary are `ssr`-gated, so the WASM hydrate surface is
  unchanged.
- `bin-target` was required by the second binary; without it `cargo-leptos`
  cannot infer which binary to build.

## Security notes

The harness generates traffic against whatever `DATABASE_URL` names. The
`write-mix` scenario deliberately uses only the idempotent get-or-create user
route, so a run does not grow the database per request. The README directs
operators to run it against a disposable database. No credential is logged;
the report contains only names, counts, statuses, and timings.

## PR review follow-up

All three review findings were addressed: schedule allocation is capped by the
requested volume; body-drain errors count as failures while retaining the
received status; and worker tasks are tracked by a `JoinSet`, which aborts on
run cancellation and is explicitly aborted and drained after a worker error.
Regression tests cover each behavior.
