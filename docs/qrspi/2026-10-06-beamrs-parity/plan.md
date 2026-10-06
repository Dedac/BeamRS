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
- CI covers format, lint, tests, and build.

## Tasks

- Add domain validation and repository support.
- Implement Axum routes and migrations.
- Add the Leptos shell and beam animation.
- Cover core flows with tests.
- Run verification and prepare the PR.
