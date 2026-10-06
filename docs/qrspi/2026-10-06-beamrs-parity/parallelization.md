---
status: approved
phase: parallelize
---

# Parallelization

No large parallel workstream is necessary for this single-crate rewrite; the codebase stays small enough for one coordinated implementation. The primary boundaries are:

- domain + repository
- server + API layer
- UI + settings page parity
- verification + CI

These are intentionally sequenced so that the database schema and repository contracts precede the server and presentation work.
