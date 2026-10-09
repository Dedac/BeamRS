---
status: approved
phase: structure
---

# Structure

## Modified files

- `beamrs/Cargo.toml`: optional `hyper-util` (`client-legacy`, `http1`,
  `tokio`) and `http-body-util` dependencies, both enabled by the `ssr`
  feature.
- `beamrs/Cargo.lock`: adds `want` and `try-lock` and changes no existing
  version.
- `beamrs/src/loadtest.rs`: `Transport`, `HttpTarget`, `serve_loopback`,
  `run_load_test_with`, the internal `Sender`, the report `transport` field,
  the `transport-error` key, and tests.
- `beamrs/src/bin/loadtest.rs`: `--transport` and `--target` flags, loopback
  hosting, the external target path, and tests.
- `README.md`: flag table, transport semantics, validation command, and
  QRSPI link.

No new source files are added. No route, schema, or UI files change.
