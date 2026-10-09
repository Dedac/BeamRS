---
status: complete
phase: research
---

# Research summary

## Q1 — HTTP client

`Cargo.lock` already resolves `hyper 1.12.0`, `hyper-util 0.1.21`, and
`http-body-util 0.1.5` through axum. Enabling `hyper-util`'s
`client-legacy`, `http1`, and `tokio` features adds a pooled keep-alive
client. The only new lockfile packages are hyper's client helpers `want` and
`try-lock`, and no existing version changes. `reqwest` would add a much
larger dependency tree.

## Q2 — Shared engine

`run_load_test` in `beamrs/src/loadtest.rs` only touches the router in one
spot, where it calls `router.clone().oneshot(request)` and then drains the
body with `to_bytes`. A hyper client response body can be wrapped in
`axum::body::Body::new`, so draining, body-error detection, the schedule, the
JoinSet cleanup, and the report all stay shared.

## Q3 — Hosting the router

`server::router(state)` is public, and `axum::serve` with a
`tokio::net::TcpListener` bound to `127.0.0.1:0` serves it on an ephemeral
port. `server::run` is unsuitable because it binds `PORT` and installs a
global tracing subscriber.

## Q4 — Connection failures

A sample with `status: None` is currently reported under
`request-build-error`. With the in-process transport `oneshot` cannot fail,
because the router's error type is `Infallible`, so a new
`transport-error` key does not change any existing output.
