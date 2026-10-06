---
status: approved
phase: design
---

# Design

## Architecture

BeamRS uses a small Rust full-stack architecture:

- `beamrs` crate contains the application, domain logic, repositories, and server.
- Leptos renders the server-side shell and UI surfaces for the feed and settings screens.
- Axum exposes REST endpoints for frequencies, rays, users, and prisms.
- PostgreSQL persists user identity, feed data, and prism state through SQLx migrations.

## Design choices

- Preserve the Beam domain model but add explicit validation helpers.
- Make uniqueness and integrity constraints part of schema design, not only repository logic.
- Keep API semantics aligned with the original features while avoiding unsafe mutation through GET requests.
- Provide demo seed data so the project is immediately usable in a Docker or local dev environment.

## Risks and mitigations

- Risk: naming collisions and anonymous IDs may create duplicates.
  - Mitigation: require unique usernames in the database and use get-or-create semantics with validation.
- Risk: invalid state transitions on prism toggles.
  - Mitigation: enforce uniqueness in `prisms (user_id, ray_id)` and treat add/remove as idempotent operations.
- Risk: visual parity drift.
  - Mitigation: keep the feed and settings layout intentionally similar to the original DSL while using original, from-scratch styles.
