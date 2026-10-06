---
status: approved
phase: questions
---

# Questions

## Q-01: What exact domain semantics must be preserved?

- Users are anonymous by default and generated as `Anon<N>`.
- Frequencies, rays, and prism relationships are persisted with integrity and uniqueness constraints.
- A user may be looked up by username and created on demand.

## Q-02: Which routes and flows are required for parity?

- Welcome view and frequency navigation
- Create frequency
- Select frequency and compose a ray
- Prism and unprism a ray
- View authored rays and prismed rays
- Update current display name from settings
- Persistent "See Myself" navigation

## Q-03: What implementation constraints matter for the Rust rewrite?

- PostgreSQL-backed data access with migrations
- Secure, explicit state changes on POST/DELETE semantics
- Demo data enabled by default for local development
- CI covering format, clippy, tests, and build
- One shared Leptos component tree for Axum SSR and Rust/WASM hydration
- No handwritten application JavaScript; browser APIs remain isolated Rust interop
