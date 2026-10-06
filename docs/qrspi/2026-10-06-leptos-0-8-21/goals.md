---
status: active
phase: goals
---

# Goals

Upgrade BeamRS from the Leptos 0.6 line to exactly Leptos 0.8.21 without
changing its product behavior or approved architecture.

## Constraints

- Preserve Axum SSR, browser hydration, routing, PostgreSQL flows, and the
  home frequency creation CTA.
- Keep the migration scoped to compatibility work; do not import changes from
  other worktrees.
- Retain the existing Leptos/Axum/cargo-leptos architecture and repository
  conventions.

## Acceptance criteria

- Direct Leptos crates resolve to 0.8.21 and the lockfile has no stale direct
  0.6 resolution.
- Rust source and build metadata compile for native SSR and WASM hydration.
- Focused validation and the repository QRSPI validator pass.
- The migration is committed with the required Copilot co-author trailer.
