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

## Review follow-up

Added `--http-timeout SEC`, defaulting to 30 seconds. It applies to both HTTP
transport modes, rejects zero, and does not change in-process behavior.
Supplying the HTTP-only flag in in-process mode is rejected with a usage
message instead of being silently ignored.
