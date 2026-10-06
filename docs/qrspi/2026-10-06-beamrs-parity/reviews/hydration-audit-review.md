---
status: approved
phase: replan
reviewed: 2026-10-06
---

# Hydration audit remediation review

## Finding

The first completion state used Leptos as an SSR string-rendering shell while a
227-line handwritten `static/app.js` implemented identity, navigation-adjacent
state, forms, REST calls, prism controls, errors, and Canvas animation. This was
a material completion gap against the approved Leptos full-stack architecture.

## Corrective decision

Keep the validated Axum/SQLx API and repository behavior, but replace the
presentation layer with:

- shared typed Leptos components and Leptos Router;
- reactive Rust signals and event handlers;
- a typed Rust client boundary for the existing REST API and local storage;
- Rust `web_sys` Canvas animation;
- cargo-leptos SSR and hydration builds.

## Acceptance evidence required

- `static/app.js` is removed.
- No application `inner_html` or string page templates remain.
- Native SSR and `wasm32` hydration targets compile and pass clippy.
- `cargo leptos build --release` emits `beamrs.js` and `beamrs.wasm`.
- Docker serves the generated assets.
- PostgreSQL-backed API and core browser shell smoke tests pass.

## Outcome

All acceptance evidence was collected. Native and hydration lint passed, all
14 tests passed, cargo-leptos generated the production bundle, PostgreSQL flows
passed against the production binary, and the final Docker image served both
generated assets successfully.
