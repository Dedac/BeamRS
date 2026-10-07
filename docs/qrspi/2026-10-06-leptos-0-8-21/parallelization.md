---
status: approved
---

# Parallelization

The migration is a single dependency and API compatibility trace, so code
changes remain sequential. Independent validations may run in parallel only
after the implementation compiles:

- QRSPI and formatting checks.
- Native SSR and WASM hydration clippy.
- PR/CI status inspection.

Database, production server, Docker, and browser checks remain ordered because
they share ports and runtime state.
