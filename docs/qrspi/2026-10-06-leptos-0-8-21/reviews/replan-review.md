---
status: complete
phase: replan
reviewed: 2026-10-06
---

# Replan record

## Loops completed

Independent integration and browser audits found two release-path gaps after
the first implementation pass:

1. The WASM entry point used a CSR mount instead of hydrating the SSR tree.
   The run looped back through design, implementation, integration, and test
   to separate the document shell from `App`, pass runtime `LeptosOptions`,
   and use `hydrate_body`.
2. The production Docker builder retained Rust 1.89, below the locked
   cargo-leptos dependency MSRV. The run looped back through research,
   implementation, and test to adopt Rust 1.96 and document the requirement.

Review feedback then identified redundant route-resource effects. They were
removed because `LocalResource` already tracks parameters read by its fetcher.

## Final decision

No further replan is required. The acceptance criteria are satisfied, the
full QRSPI route is recorded, and the migration is ready for review.
