---
status: complete
phase: implement
task: task-01-engine-http
---

# Task 1 — Engine HTTP transport

## Scope

`beamrs/src/loadtest.rs`, `beamrs/Cargo.toml`, and `beamrs/Cargo.lock`.

## Outcome

Implemented as designed. Five new tests cover loopback round-trips, JSON
bodies, the transport-error path, target validation, and the in-process
label. All existing tests pass unchanged.
