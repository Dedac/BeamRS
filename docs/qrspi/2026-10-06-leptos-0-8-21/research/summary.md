---
status: complete
phase: research
---

# Research summary

The approved `2026-10-06-beamrs-parity` run established a shared Leptos
component tree, Axum SSR, WASM hydration, and cargo-leptos production builds.
This migration starts from that approved state and does not replace it.

## Baseline inventory

- Direct Leptos crates: `leptos`, `leptos_axum`, `leptos_meta`, and
  `leptos_router`, all resolving to 0.6.15.
- Transitive Leptos crates observed: `leptos_config`, `leptos_dom`,
  `leptos_macro`, `leptos_reactive`, `leptos_server`,
  `leptos_integration_utils`, and `leptos_hot_reload`.
- Leptos-facing source is concentrated in `beamrs/src/app.rs`, `lib.rs`, and
  `server.rs`; build and release surfaces are `Cargo.toml`, `Cargo.lock`,
  `.github/workflows/beamrs-ci.yml`, `beamrs/Dockerfile`, and `README.md`.

## Compatibility risks

- Leptos 0.8 renamed or removed several legacy reactive constructors and
  changed router component APIs; compiler diagnostics must drive any source
  edits.
- `leptos_axum` route generation and `LeptosRoutes` may require signature or
  trait import adjustments.
- cargo-leptos and Rust/WASM lockfile versions must remain aligned.
