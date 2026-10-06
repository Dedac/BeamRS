---
status: approved
phase: design
---

# Design

Keep the existing application architecture intact. Upgrade the four direct
Leptos crates together to `0.8.21`, refresh the lockfile, and make only
compiler-required API edits. Preserve feature forwarding for `ssr` and
`hydrate`, cargo-leptos metadata, generated asset paths, Axum routes, and
the application component tree.

Validation must cover formatting, native clippy, WASM clippy, all-target
tests, the production cargo-leptos build, and `scripts/validate-qrspi.sh`.
