---
status: approved
pipeline: full
run: 2026-10-06-beamrs-parity
---

# QRSPI run configuration

## Route

```yaml
route:
  - goals
  - questions
  - research
  - design
  - phasing
  - structure
  - plan
  - parallelize
  - implement
  - integrate
  - test
  - replan
```

## Review policy

- Human approval is required before each downstream phase.
- Quick-fix work may use `research -> plan -> implement -> test`.
- Architectural changes require a backward loop.

## Run status

Approved scope: feature parity with Dedac/Beam using a Rust stack centered on Leptos + Axum + PostgreSQL.

The run was reopened after a completion audit found that browser behavior had
been implemented in handwritten JavaScript rather than hydrated Leptos. The
approved architecture is now enforced with cargo-leptos SSR/hydration builds.
