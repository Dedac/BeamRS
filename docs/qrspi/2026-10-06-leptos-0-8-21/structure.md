---
status: approved
---

# Structure

- `beamrs/Cargo.toml`: exact dependency versions, features, and cargo-leptos metadata.
- `beamrs/Cargo.lock`: resolved migration dependency graph.
- `beamrs/src/lib.rs`: hydration entrypoint and compile recursion limit.
- `beamrs/src/app.rs`: shared document shell, router, pages, and browser behavior.
- `beamrs/src/server.rs`: Axum integration, SSR routes, static fallback, and tests.
- `beamrs/Dockerfile`: cargo-leptos release build and runtime asset packaging.
- `.github/workflows/beamrs-ci.yml`: native, WASM, test, and release validation.

No new runtime subsystem is introduced. The migration updates the existing
boundaries to their Leptos 0.8 contracts.
