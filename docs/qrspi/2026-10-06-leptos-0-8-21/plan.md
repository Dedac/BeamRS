---
status: approved
phase: plan
---

# Plan

1. Change direct Leptos dependencies to exactly `0.8.21` and refresh the
   lockfile.
2. Compile native SSR and WASM hydration to identify required API changes.
3. Apply the smallest source/build/documentation changes that preserve current
   behavior.
4. Run all requested validation commands, including the QRSPI validator.
5. Review, commit, and record final migration issues and validation results.
