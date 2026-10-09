---
status: resolved
phase: questions
---

# Questions

1. What is the narrowest seam in the existing codebase through which a load
   generator can exercise real application behavior?
2. Can a load generator be built from dependencies already in the lockfile,
   or does it require a new HTTP client crate?
3. How do the existing tests drive the application, and can the harness reuse
   that mechanism so its own tests need no database?
4. What does adding a second binary target do to the `cargo-leptos` build,
   and what metadata keeps that build unambiguous?
5. Which measurements make a load report actionable rather than decorative?

## Resolution approach

Read `beamrs/src/server.rs`, `beamrs/src/repository.rs`, `beamrs/Cargo.toml`,
and the existing test module; confirm buildability with an offline
`cargo check`; consult `cargo-leptos` metadata conventions for multi-binary
packages.
