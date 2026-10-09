---
status: complete
phase: test
---

# Test review

## Automated validation

| Command | Result |
|---|---|
| `./scripts/validate-qrspi.sh docs/qrspi/2026-10-09-http-load-testing` | pass |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets --features ssr -- -D warnings` | pass |
| `cargo clippy --lib --no-default-features --features hydrate --target wasm32-unknown-unknown -- -D warnings` | pass |
| `cargo test --all-targets` | pass: 38 lib tests, including 5 new transport tests, and 13 CLI tests, including 3 new |

## Measurement (G1, G2)

All runs used a release build with the `read-mix` scenario against a
disposable `postgres:16-alpine` container on 127.0.0.1, on an Apple M5 Max
laptop. Every run had zero failures.

| Mode | Concurrency | Requests | req/s | p50 ms | p95 ms | p99 ms |
|---|---|---|---|---|---|---|
| in-process (previous run, baseline) | 12 | 200,000 | 10,226 | 1.26 | 1.60 | 1.86 |
| `--transport http` (loopback) | 14 | 200,000 | 10,286 | 1.45 | 1.86 | 2.14 |
| `--target` running `beamrs` server | 12 | 200,000 | 10,818 | 1.18 | 1.57 | 1.83 |

Loopback sweep with 50,000 requests per step: 8,523 req/s at c=8, 10,680 at
c=16, 14,033 at c=32, and 19,597 at c=64. The external-server sweep
plateaued at about 11.5k req/s from c=24 upward, which matches the
server's fixed `max_connections(20)` database pool in `server::run`. The
harness's own pool scales with concurrency up to 64 connections.

The harness is closed-loop, so these figures are achieved throughput at a
given concurrency, not a fixed arrival rate.
