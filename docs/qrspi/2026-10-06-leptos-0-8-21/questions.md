---
status: approved
---

# Questions

1. Which exact compatible versions accompany Leptos 0.8.21?
2. Which component, router, meta, hydration, and Axum APIs changed from 0.6?
3. Does one shared component tree still provide SSR and browser hydration?
4. Do the existing REST routes and PostgreSQL behavior remain unchanged?
5. Which cargo-leptos release reproducibly packages the required runtime assets?

## Resolutions

The selected ecosystem is Leptos 0.8.21, leptos_axum 0.8.9,
leptos_meta 0.8.6, and leptos_router 0.8.16. Compiler errors, tests, generated
assets, runtime checks, and browser checks are the authority for API and
behavior compatibility. cargo-leptos 0.3.11 is required so its wasm-bindgen
schema matches the current resolved WASM dependency graph in clean builds.
