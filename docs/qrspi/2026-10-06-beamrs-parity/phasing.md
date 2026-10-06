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

- Render the home, feed, settings, and user profile screens
- Preserve current-user behavior and "See Myself"
- Add the animated beam effect

## Phase 4: verification

- Run format, clippy, tests, and build
- Validate parity with targeted API and repository tests
- Record final review and prepare PR
