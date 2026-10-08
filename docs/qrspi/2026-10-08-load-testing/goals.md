---
status: approved
phase: goals
---

# Goals

Give BeamRS a repeatable way to measure how the application behaves under
concurrent request load, so performance regressions are observable before
deployment rather than in production.

## Constraints

- Preserve all existing product behavior, routes, schema, and UI.
- Add no new third-party dependency; the repository builds offline from the
  committed lockfile.
- Keep the harness usable in CI and locally, with deterministic test coverage
  that does not require a database.
- Follow existing repository conventions: Rust 2021, `cargo fmt`, clippy with
  `-D warnings`, unit tests colocated with the module.

## Acceptance criteria

- A load profile with configurable concurrency, request count, and weighted
  endpoint mix can be run against the real application router.
- A run reports total requests, successes, failures, HTTP status counts,
  throughput, and latency percentiles overall and per endpoint.
- Invalid profiles (zero concurrency, zero requests, empty or zero-weight
  endpoint mix, malformed path) are rejected with a clear error instead of
  producing a misleading report.
- An operator-facing entry point exists and is documented in `README.md`.
- Focused validation passes: `cargo fmt --check`, clippy with `-D warnings`,
  `cargo test --all-targets`, and `scripts/validate-qrspi.sh`.
