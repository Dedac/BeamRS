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
- The browser entry point must call `hydrate_body`, not the CSR-only
  `mount_to_body`; an independent browser audit found the latter duplicated
  the entire SSR component tree instead of attaching to it.
- `LocalResource` tracks route parameters read by its fetcher. Redundant mount
  effects were removed so frequency and profile routes do not issue duplicate
  initial API requests.
- `cargo-leptos 0.3.11` is required for the `wasm-bindgen 0.2.129` schema
  emitted by the Leptos 0.8 dependency graph.
- The Docker builder was raised from Rust 1.89 to Rust 1.96 because
  `cargo-leptos 0.3.11 --locked` includes `cargo-util-schemas 0.14.2`, whose
  MSRV is Rust 1.96. An independent clean image build exposed and corrected
  this release-path gap.

## Validation

- `cargo fmt --all -- --check`
- Native SSR clippy with `-D warnings`
- WASM hydrate clippy with `-D warnings`
- `cargo test --all-targets` (14 passed)
- `cargo leptos build --release`
- PostgreSQL 16 smoke test covering startup migrations, SSR shell generation,
  hydration asset routing, the frequency API, and a clean production Docker
  image build using the documented toolchain floor
- Browser smoke test asserting one hydrated component tree, responsive
  create-frequency focus and navigation, the frequency composer, and settings
- `./scripts/validate-qrspi.sh docs/qrspi/2026-10-06-leptos-0-8-21`
