---
status: complete
phase: research
---

# Research summary

## Q1 — Narrowest seam for a load generator

`beamrs/src/server.rs` exposes `pub fn router(state: AppState) -> Router`,
which assembles the health route, the JSON API, the Leptos SSR routes, and
the static-file fallback. `AppState` holds `repo: Arc<dyn BeamStore>` and the
Leptos options, so a caller can build the full router with either the real
`BeamRepository` or any other `BeamStore` implementation. Driving that
`Router` directly exercises extractors, handlers, error mapping, and the
repository path.

## Q2 — Dependencies already available

`tower` (with `ServiceExt::oneshot`), `axum`, `tokio` (multi-thread runtime),
`serde`/`serde_json`, `thiserror`, `anyhow`, and `dotenvy` are already
`ssr` dependencies. `axum::body::to_bytes` can drain a response body, so no
HTTP client crate is needed. An offline `cargo check --features ssr`
succeeds, confirming the lockfile is complete for this dependency set.

## Q3 — Existing test mechanism

The test module in `beamrs/src/server.rs` builds an in-memory `MemoryStore`
implementing `BeamStore`, constructs the router, and calls
`router.oneshot(request)` with `http-body-util` for body collection. The same
`oneshot` mechanism is available to non-test code through `tower::ServiceExt`,
so a harness can be unit-tested against a throwaway `axum::Router` without
PostgreSQL.

## Q4 — Second binary and cargo-leptos

`cargo-leptos` builds the server binary from the package. With more than one
binary target it cannot infer which to build, and the
`[package.metadata.leptos] bin-target` key disambiguates it. Explicit
`[[bin]]` entries with `required-features = ["ssr"]` keep the new binary out
of the `hydrate`/WASM configuration, which is checked with `--lib` only.

## Q5 — Actionable measurements

A useful report needs volume (total/success/failure), correctness signal
(status-code histogram), rate (requests per second over wall-clock elapsed),
and tail behavior (mean plus p50/p95/p99/max). Per-endpoint breakdown matters
because a mixed profile hides which route is slow. Nearest-rank percentiles
over the full sample vector are exact for these run sizes and need no
estimator.

## Caveat carried forward

In-process generation measures handler and store latency; it excludes kernel
networking, TLS, and HTTP parsing. The documentation must state this so
numbers are not mistaken for end-to-end client latency.
