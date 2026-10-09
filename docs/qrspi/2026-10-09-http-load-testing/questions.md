---
status: resolved
phase: questions
---

# Questions

1. Which HTTP client can the harness use without adding new top-level crates
   or upgrading existing versions?
2. Can the existing engine's worker, schedule, and report logic stay shared
   between the transports?
3. How should the harness host the router on a socket without duplicating
   `server::run` startup, which installs a global tracing subscriber and
   binds a fixed port?
4. How do connection-level failures appear in the existing report model?
