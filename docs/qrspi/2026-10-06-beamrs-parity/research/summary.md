# Research summary

This run confirms the Beam domain and functional contract from the public repository, and it maps that contract onto a Rust architecture with PostgreSQL persistence and Leptos/Axum SSR plus browser hydration.

## Major conclusions

1. The domain is intentionally small and cohesive: `User`, `Frequency`, `Ray`, and `Prism` map cleanly to relational tables and repository APIs.
2. The original app depends on anonymous identity generation and get-or-create user behavior, which should be preserved but documented as a user-privacy choice with explicit naming semantics.
3. Safe server behavior requires POST-based mutation routes rather than GET-based mutating actions; the rewrite should keep the old capability but enforce correct HTTP semantics.
4. The original visual theme is a pun/social feed with a simple animated beam, so a Rust reimplementation can reuse the spirit without copying source or assets.
5. Local usability requires migrations and demo seed data so the application is immediately demonstrable.
6. Leptos full-stack parity requires a cargo-leptos-generated Rust/WASM
   hydration bundle; an SSR shell with handwritten application JavaScript does
   not meet the selected architecture.
