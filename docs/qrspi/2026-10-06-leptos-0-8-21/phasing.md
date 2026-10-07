---
status: approved
---

# Phasing

1. Confirm the clean main-based branch and inspect the prior worktree read-only.
2. Align Cargo dependencies and cargo-leptos metadata.
3. Migrate shared components, router, document shell, and hydration entrypoint.
4. Migrate Axum/Leptos server setup and tests.
5. Integrate release assets and container behavior.
6. Run QRSPI, format, clippy, tests, release, runtime, and browser validation.
7. Replan on any compiler, runtime, or hydration failure.
8. Commit, push, and open an unmerged pull request.
