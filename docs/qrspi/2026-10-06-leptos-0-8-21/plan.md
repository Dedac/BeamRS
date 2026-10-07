---
status: approved
---

# Plan

1. Pin Leptos to 0.8.21 and align its integration crates.
2. Update route declarations, view construction, signals, and browser interop.
3. Render the complete HTML document with meta and hydration assets.
4. Update Leptos/Axum options, route registration, and static file fallback.
5. Adapt tests to the local reactive runtime contract.
6. Run all required checks and inspect generated JS/WASM.
7. Start PostgreSQL and the production/container server; exercise API and UI.
8. Record results, commit with the required trailer, push, and keep the PR open.

## Replan rule

Any failure returns to the owning phase. Compiler failures return to
implementation; missing generated assets return to design/integration; browser
behavior failures return to the shared component and hydration implementation.
