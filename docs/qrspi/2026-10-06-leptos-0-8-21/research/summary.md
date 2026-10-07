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

## Resolved migration inventory

- Direct crates: `leptos 0.8.21`, `leptos_axum 0.8.10`,
  `leptos_dom 0.8.8`, `leptos_meta 0.8.7`, and
  `leptos_router 0.8.16`.
- Transitive Leptos crates: `leptos_config 0.8.10`,
  `leptos_hot_reload 0.8.7`, `leptos_integration_utils 0.8.9`,
  `leptos_macro 0.8.19`, `leptos_router_macro 0.8.7`, and
  `leptos_server 0.8.8`.
- Shared reactive/rendering infrastructure: `reactive_graph 0.2.15`,
  `server_fn 0.8.13`, and `tachys 0.2.19`.
- Build tooling: `cargo-leptos 0.3.11`, aligned in local documentation, CI,
  and the production Docker image.
- Toolchain floor: `cargo-leptos 0.3.11 --locked` resolves
  `cargo-util-schemas 0.14.2`, which requires Rust 1.96. The production
  builder and documented prerequisites therefore use Rust 1.96 or newer.

## Compatibility risks

- Leptos 0.8 renamed or removed several legacy reactive constructors and
  changed router component APIs; compiler diagnostics must drive any source
  edits.
- `leptos_axum` route generation and `LeptosRoutes` may require signature or
  trait import adjustments.
- cargo-leptos and Rust/WASM lockfile versions must remain aligned.
- The Docker builder toolchain must satisfy cargo-leptos's locked dependency
  MSRV; retaining the previous Rust 1.89 image prevents the image from
  building.
