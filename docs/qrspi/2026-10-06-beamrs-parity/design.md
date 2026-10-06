---
status: approved
phase: design
---

# Design

## Architecture

BeamRS uses a small Rust full-stack architecture:

- `beamrs` crate contains the application, domain logic, repositories, and server.
- Leptos renders one shared component tree on the server and hydrates it in the
  browser from a Rust `wasm32-unknown-unknown` bundle produced by cargo-leptos.
- Leptos Router and reactive signals own navigation, forms, identity, REST
  interactions, prism state, and loading/error/empty UI.
- Browser-only APIs are isolated in Rust: `gloo-net` for REST, `web_sys`
  local storage, and a `web_sys` Canvas 2D animation.
- Axum exposes REST endpoints for frequencies, rays, users, and prisms.
- PostgreSQL persists user identity, feed data, and prism state through SQLx migrations.

## Design choices

- Preserve the Beam domain model but add explicit validation helpers.
- Make uniqueness and integrity constraints part of schema design, not only repository logic.
- Keep API semantics aligned with the original features while avoiding unsafe mutation through GET requests.
- Provide demo seed data so the project is immediately usable in a Docker or local dev environment.
- Do not maintain a handwritten application JavaScript layer; generated
  wasm-bindgen bootstrap output is the only JavaScript shipped by the app.

## Risks and mitigations

- Risk: naming collisions and anonymous IDs may create duplicates.
  - Mitigation: require unique usernames in the database and use get-or-create semantics with validation.
- Risk: invalid state transitions on prism toggles.
  - Mitigation: enforce uniqueness in `prisms (user_id, ray_id)` and treat add/remove as idempotent operations.
- Risk: visual parity drift.
  - Mitigation: keep the feed and settings layout intentionally similar to the original DSL while using original, from-scratch styles.
- Risk: an SSR-only shell could superficially satisfy Leptos while leaving the
  browser application in JavaScript.
  - Mitigation: CI builds and clippies both the SSR and hydration targets, and
    smoke tests assert that the generated WASM loader is served.
