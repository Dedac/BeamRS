---
status: complete
phase: test
---

# Migration review

## Result

BeamRS now resolves core Leptos `0.8.21` with compatible companion releases:
`leptos_axum 0.8.10`, `leptos_meta 0.8.7`, and `leptos_router 0.8.16`.
The approved parity run at `docs/qrspi/2026-10-06-beamrs-parity/` was not
modified.

## Compatibility issues resolved

- Leptos 0.8 reactive constructors, effects, resources, router paths, mount
  APIs, and configuration builders replaced their 0.6 forms.
- Axum 0.8 required `{parameter}` route captures and compatible tower-http
  releases.
- Leptos Meta SSR requires a document head and `MetaTags`.
- Hydration requires `HydrationScripts` in the application shell.
- `cargo-leptos 0.3.11` is required for the `wasm-bindgen 0.2.129` schema
  emitted by the Leptos 0.8 dependency graph.

## Validation

- `cargo fmt --all -- --check`
- Native SSR clippy with `-D warnings`
- WASM hydrate clippy with `-D warnings`
- `cargo test --all-targets` (14 passed)
- `cargo leptos build --release`
- `./scripts/validate-qrspi.sh docs/qrspi/2026-10-06-leptos-0-8-21`
