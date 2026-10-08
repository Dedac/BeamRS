---
status: approved
phase: structure
---

# Structure

## New files

- `beamrs/src/loadtest.rs` — engine, value types, report rendering, and the
  module's unit tests.
- `beamrs/src/bin/loadtest.rs` — operator CLI: flag parsing, scenario
  definitions, PostgreSQL wiring, report output, exit status.

## Modified files

- `beamrs/src/lib.rs` — declare `pub mod loadtest;` under `#[cfg(feature =
  "ssr")]`.
- `beamrs/Cargo.toml` — explicit `[[bin]]` targets with
  `required-features = ["ssr"]`, and `bin-target = "beamrs"` under
  `[package.metadata.leptos]` so the cargo-leptos build stays unambiguous.
- `README.md` — load-testing section covering usage, scenarios, output, and
  the in-process measurement caveat.

## Interfaces

```rust
pub struct RequestSpec { name, method, path, body, weight }
pub struct LoadProfile { concurrency, total_requests, requests }
pub struct LatencyStats { count, failures, mean_ms, p50_ms, p95_ms, p99_ms, max_ms }
pub struct LoadReport { elapsed_ms, total_requests, successes, failures,
                        requests_per_second, status_counts, overall, endpoints }

impl LoadProfile { pub fn validate(&self) -> Result<(), LoadTestError>; }
pub async fn run_load_test(router: Router, profile: &LoadProfile)
    -> Result<LoadReport, LoadTestError>;
impl LoadReport { pub fn render_text(&self) -> String; }
```

## Boundaries

`loadtest` depends on `axum`, `tower`, `tokio`, `serde`, and `thiserror`
only. It does not import `repository`, `config`, or `app`; the binary owns
that wiring. Nothing in the existing request path imports `loadtest`.
