---
status: complete
phase: test
---

# Test review

## Automated validation

| Command | Result |
|---|---|
| `./scripts/validate-qrspi.sh docs/qrspi/2026-10-08-load-testing` | pass — "QRSPI run structure is valid" |
| `cargo fmt --all -- --check` | pass — no diff |
| `cargo clippy --all-targets --features ssr -- -D warnings` | pass — no warnings |
| `cargo clippy --lib --no-default-features --features hydrate --target wasm32-unknown-unknown -- -D warnings` | pass — no warnings |
| `cargo test --all-targets` | pass — 29 lib tests, 0 main tests, 10 `loadtest` binary tests; 39 passed, 0 failed |
| `cargo leptos build --release` | pass — built `--bin=beamrs` and the WASM bundle |

23 of the 39 tests are new (13 engine, 10 binary). The 16 pre-existing tests
are unchanged and still pass.

## Manual acceptance against goals

Run against `docker compose up -d db` with
`DATABASE_URL=postgres://beamrs:beamrs@localhost:5432/beamrs`.

- `--scenario read-mix --concurrency 8 --requests 200` → 200 requests, 0
  failures, 1638.3 req/s, statuses `200=180 204=20`, per-endpoint rows for
  `frequency-rays`, `health`, `list-frequencies`, `user-prisms`, `user-rays`
  in the documented weight ratio 3:1:3:1:2. Exit status 0.
- `--scenario write-mix --concurrency 8 --requests 120 --username LoadProbe`
  → 120 requests, 0 failures, 1892.5 req/s, `upsert-user` present with 45
  requests. Exit status 0.
- `--scenario health --concurrency 2 --requests 10 --json` → well-formed JSON
  report with overall and per-endpoint statistics. Exit status 0.
- `--help` prints usage and exits 0; `--nope`, a missing flag value, a
  non-numeric count, and an unknown scenario are each rejected with a message
  and exit status 1.

Every acceptance criterion in `goals.md` is met.

## Known limitation

Latency is measured in-process, so it excludes networking and HTTP wire
parsing. This is stated in `README.md` and in the module documentation.
