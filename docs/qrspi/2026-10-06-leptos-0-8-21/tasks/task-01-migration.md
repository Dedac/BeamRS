---
status: complete
---

# Task 01: Leptos 0.8.21 migration

## Scope

Upgrade Cargo dependencies and migrate the shared application, hydration
entrypoint, Axum server integration, static asset fallback, and affected tests.

## Acceptance

- Exact Leptos version is 0.8.21.
- SSR and WASM hydration compile with warnings denied.
- Existing tests and API behavior pass.
- The production document loads cargo-leptos JavaScript and WASM.
- Required browser flows remain functional.
