---
status: approved
---

# Research summary

Leptos 0.8 requires updated route ownership, document shell/meta integration,
typed server options, and current Axum route registration. The app must render
`MetaTags` and `HydrationScripts` in the SSR document so cargo-leptos assets are
loaded and hydrate the same component tree.

The compatible release line used by this migration is:

- `leptos = =0.8.21` (an exact Cargo requirement)
- `leptos_axum = =0.8.9`
- `leptos_meta = =0.8.6`
- `leptos_router = =0.8.16`
- `axum = 0.8`
- `tower = 0.5`
- `cargo-leptos = 0.3.11`

cargo-leptos 0.2.34 embeds wasm-bindgen schema 0.2.100 and fails a clean Docker
build against the current dependency graph. Version 0.3.11 matches the generated
WASM schema and is used consistently by local documentation, CI, and Docker.
The Docker builder tracks Rust 1.99, matching the stable toolchain used to
install cargo-leptos 0.3.11 and avoiding the obsolete Rust 1.89 toolchain.

The previous dirty worktree was reference-only. The implementation in this
branch was independently checked through the compiler, tests, release output,
container runtime, and browser behavior.
