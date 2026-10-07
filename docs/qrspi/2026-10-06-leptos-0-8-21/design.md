---
status: approved
---

# Design

## Architecture

Keep the existing full-stack boundary:

- Axum owns HTTP, REST routes, static assets, health checks, and PostgreSQL.
- One typed Leptos component tree renders on the server and hydrates in WASM.
- Leptos Router owns browser navigation.
- `gloo-net` and `web_sys` remain hydrate-only browser dependencies.
- cargo-leptos builds the native binary, stylesheet, JavaScript loader, and WASM.

## Migration decisions

- Use the exact Leptos 0.8.21 release with compatible ecosystem crates.
- Update APIs in place rather than introducing a parallel frontend.
- Preserve route and JSON contracts.
- Use the Leptos 0.8 document shell with `MetaTags` and `HydrationScripts`.
- Hydrate the server-rendered body with `hydrate_body`; never mount a second app.
- Keep the Canvas animation callback alive while settings is mounted and cancel it
  during reactive cleanup.
- Keep server tests inside a Tokio `LocalSet` where the reactive runtime uses
  local tasks.

## Exclusions

Authentication redesign and unrelated security-alert remediation are outside
this migration.
