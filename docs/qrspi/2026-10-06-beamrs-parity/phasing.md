---
status: approved
phase: phasing
---

# Phasing

## Phase 1: foundation and schema

- Create the Rust crate and dependencies
- Add PostgreSQL migrations and seed data
- Implement domain validation and repository contracts

## Phase 2: server and API parity

- Expose frequencies, users, rays, and prism endpoints in Axum
- Keep SQL semantics explicit and idempotent
- Add demo data and configuration support

## Phase 3: UI parity

- Render the home, feed, settings, and user profile screens as shared Leptos components
- Hydrate Leptos Router, signals, forms, identity, API calls, and prism controls from Rust/WASM
- Preserve current-user behavior and "See Myself"
- Add the animated beam effect through Rust `web_sys` Canvas APIs

## Phase 4: verification

- Run format, native and WASM clippy, tests, and a cargo-leptos production build
- Validate parity with targeted API and repository tests
- Build Docker and smoke-test the generated browser bundle against PostgreSQL
- Record final review and prepare PR
