---
status: draft
pipeline: full
run: replace-with-run-id
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

