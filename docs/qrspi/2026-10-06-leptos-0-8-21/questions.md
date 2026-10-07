---
status: resolved
phase: questions
---

# Questions

1. Which direct and transitive Leptos packages are present, and which APIs
   changed between 0.6 and 0.8?
2. Does the current `leptos_axum` route integration remain source-compatible?
3. Do feature forwarding, cargo-leptos metadata, Docker, and CI require
   coordinated version changes?
4. Can the migration preserve the existing browser and PostgreSQL behavior
   without unrelated refactoring?

## Resolution approach

Inventory with `cargo tree`, source search, and lockfile inspection; then use
the compiler and repository validation commands as the compatibility oracle.
