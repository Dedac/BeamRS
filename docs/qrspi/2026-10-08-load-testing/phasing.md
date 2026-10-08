---
status: complete
phase: phasing
---

# Phasing

This change is one vertical slice delivered in a single phase.

1. **Phase 1 — Load-testing capability.** Engine module, operator binary,
   build metadata, unit tests, and documentation. The slice is complete only
   when an operator can run a scenario and read a report, and when the engine
   is covered by database-free tests.

No later phase is planned. Possible follow-ups (duration-bounded runs, rate
limiting, a CI performance budget) are deliberately out of scope and recorded
in `reviews/phase-review.md`.
