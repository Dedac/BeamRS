---
status: approved
phase: plan
---

# Plan

## Acceptance criteria

- The application builds with rustfmt/clippy passing.
- PostgreSQL migrations support the Beam domain.
- Users can create and resolve anonymous usernames via get-or-create semantics.
- Frequencies can be created and selected.
- Rays can be created, listed, and prismed/unprismed.
- A user profile and settings screen exist.
- A demo-ready seed dataset is installed.
- The browser UI is a hydrated Leptos application compiled to WASM, with no
  handwritten application JavaScript.
- CI covers format, native/WASM lint, tests, production full-stack build, and
  a generated-bundle smoke assertion.

## Tasks

- Add domain validation and repository support.
- Implement Axum routes and migrations.
- Add shared Leptos components, Router, reactive browser flows, and Rust Canvas animation.
- Cover core flows with tests.
- Run verification and prepare the PR.
