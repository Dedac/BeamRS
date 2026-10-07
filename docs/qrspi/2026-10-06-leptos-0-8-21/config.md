---
status: approved
pipeline: full
run: 2026-10-06-leptos-0-8-21
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

## Approval record

The user approved an end-to-end HydraFusion Cascade migration from Leptos 0.6
to exactly 0.8.21. The escalation required independent verification of any
existing work, full validation, a pushed branch, and an unmerged pull request.

This run was recorded during the completion audit because the initial migration
commit incorrectly reused the older parity run. No approval gate is silently
skipped: the original user instruction is the scope approval, and the review
records capture the design, integration, and test gates.
