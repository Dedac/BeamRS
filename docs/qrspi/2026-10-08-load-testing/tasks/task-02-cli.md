---
status: complete
phase: implement
task: task-02-cli
---

# Task 2 — Operator binary

## Scope

`beamrs/src/bin/loadtest.rs`, the `[[bin]]` targets in `beamrs/Cargo.toml`,
and `bin-target = "beamrs"` under `[package.metadata.leptos]`.

## Outcome

The binary parses flags, builds a scenario profile, connects to PostgreSQL,
constructs the real router, runs the load test, and prints a text or JSON
report. Exit status is 1 when any request fails, so the harness can gate a
pipeline.

Both binaries declare `required-features = ["ssr"]`, which keeps them out of
the WASM hydrate configuration. `bin-target` keeps the `cargo-leptos` build
unambiguous now that the package has two binaries.

`LEPTOS_OUTPUT_NAME` is defaulted to the package output name when unset,
because this binary is not launched by `cargo-leptos` and would otherwise emit
a spurious configuration warning.

## Tests

10 unit tests cover defaults, every flag, help, unknown option, missing value,
non-numeric value, unknown scenario, profile validity for all three scenarios,
percent-encoded usernames in paths, and the write-mix idempotency constraint.
