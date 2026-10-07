---
status: approved
phase: integration
reviewed: 2026-10-06
---

# Integration review

- The server and browser use the same Leptos component tree.
- The browser hydrates the SSR body in place without duplicating the app shell.
- The SSR document includes Leptos meta and hydration assets.
- Axum registers Leptos routes and serves generated assets from the site root.
- REST route shapes and PostgreSQL storage behavior are unchanged.
- Hydrate-only crates remain behind the `hydrate` feature.
- Server-only crates remain behind the `ssr` feature.
- Docker packages the native binary and generated `target/site` output.

No integration dependency on the dirty reference worktree exists.
