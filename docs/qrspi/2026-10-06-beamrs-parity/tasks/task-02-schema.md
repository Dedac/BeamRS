---
status: complete
---

# Task 02: Database schema and migrations

Create PostgreSQL migrations for the Beam schema and demo data to make the app immediately runnable.

## Result

Added embedded SQLx migrations with foreign keys, uniqueness, validation
checks, query indexes, idempotent prism integrity, and immediately useful demo
data. Startup applies migrations before accepting requests.
