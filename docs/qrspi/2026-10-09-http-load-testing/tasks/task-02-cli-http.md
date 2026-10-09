---
status: complete
phase: implement
task: task-02-cli-http
---

# Task 2 — CLI flags

## Scope

`beamrs/src/bin/loadtest.rs`.

## Outcome

Added `--transport` and `--target`. The loopback server is aborted after the
run. In `--target` mode the database connection is skipped. Three new parser
tests were added.
