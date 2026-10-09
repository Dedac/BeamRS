# BeamRS

BeamRS is a Rust reimplementation of [Dedac/Beam](https://github.com/Dedac/Beam),
preserving its anonymous, frequency-based social experience with a new
Leptos + Axum + PostgreSQL architecture.

Users receive a browser-local `Anon<N>` identity, can retune that display name,
create and browse frequencies, transmit short rays, prism/unprism rays, and
view authored or prismed rays on user profiles. The settings page includes an
animated canvas beam and visible pass counter.

## Architecture

```text
Browser
  ├── hydrated Leptos router and reactive components compiled to WASM
  ├── Rust client module for local identity and REST calls
  ├── web_sys Canvas animation
  └── accessible responsive CSS
          │
          ▼
Axum router / REST API
          │
          ▼
BeamStore boundary
          │
          ▼
SQLx PostgreSQL repository
```

The server and browser render the same Leptos component tree. `cargo-leptos`
builds the Axum SSR binary and the hydration library as a `wasm32` bundle;
there is no handwritten application JavaScript. Rust reactive signals own
navigation, forms, loading/error/empty states, identity, and prism controls.
The browser-only layer uses `gloo-net` for the existing REST API and `web_sys`
for local storage and Canvas. Axum owns SSR, generated assets, health checks,
and the JSON API. SQLx runs embedded migrations at startup.

## Prerequisites

- Rust 1.96 or newer
- `wasm32-unknown-unknown`
- `cargo-leptos` 0.3.11
- PostgreSQL 16+, or Docker with Docker Compose

## Run locally

```sh
cd beamrs
cp .env.example .env
docker compose up -d db
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --version 0.3.11 --locked
cargo leptos watch
```

Open <http://localhost:8080>. The application automatically applies migrations
and installs demo frequencies, users, rays, and a prism on a new database.

To use an existing PostgreSQL instance, set:

```sh
export DATABASE_URL=postgresql://beamrs:beamrs@localhost:5432/beamrs
export HOST=127.0.0.1
export PORT=8080
cd beamrs
cargo leptos watch
```

`DATABASE_URL` is required. `HOST` defaults to `0.0.0.0`, and `PORT` defaults
to `8080`.

## Docker workflow

Run the complete application and database:

```sh
cd beamrs
docker compose up --build
```

Stop containers while preserving the database:

```sh
docker compose down
```

Add `--volumes` only when you intentionally want to delete local database data.

## API

| Method | Route | Purpose |
|---|---|---|
| `GET` | `/api/frequencies` | List frequencies |
| `POST` | `/api/frequencies` | Create a frequency |
| `GET` | `/api/frequencies/:id/rays` | List rays on a frequency |
| `POST` | `/api/users` | Get or create a user by username |
| `PATCH` | `/api/users/:id` | Change a display name |
| `GET` | `/api/users/:username/rays` | List authored rays |
| `GET` | `/api/users/:username/prisms` | List prismed rays |
| `POST` | `/api/rays` | Transmit a ray |
| `POST` | `/api/prisms` | Prism a ray idempotently |
| `DELETE` | `/api/prisms/:user_id/:ray_id` | Remove a prism idempotently |

State changes never use `GET`. JSON errors use an `{"error":"..."}` body and
an appropriate HTTP status.

## Load testing

BeamRS ships a `loadtest` binary that drives the real Axum router under
configurable concurrency and reports throughput and latency percentiles.

```sh
cd beamrs
docker compose up -d db
export DATABASE_URL=postgres://beamrs:beamrs@localhost:5432/beamrs
cargo run --bin loadtest -- --scenario read-mix --concurrency 8 --requests 200
```

| Flag | Default | Purpose |
|---|---|---|
| `--scenario` | `read-mix` | `health`, `read-mix`, or `write-mix` |
| `--concurrency` | `16` | Concurrent workers |
| `--requests` | `200` | Total requests to send |
| `--frequency-id` | `1` | Frequency used by read requests |
| `--username` | `Anon1` | User used by profile requests |
| `--json` | off | Print the report as JSON instead of text |

Scenarios are weighted request mixes: `health` hits the health route only,
`read-mix` exercises frequency, ray, and prism listings, and `write-mix` adds
the idempotent get-or-create user route so the write path is covered without
growing the database on every request. Requests are generated from a
deterministic weighted schedule, so repeated runs send the same mix.

```text
BeamRS load test
  concurrency      8
  requests         200
  elapsed          122.1 ms
  throughput       1638.3 req/s
  successes        200
  failures         0
  status counts    200=180 204=20

endpoint                       count   failed   mean ms    p50 ms    p95 ms    p99 ms    max ms
(all)                            200        0      4.84      0.82      2.59    117.22    121.82
frequency-rays                    60        0      6.38      0.88      2.89    117.25    117.25
health                            20        0      0.01      0.01      0.02      0.02      0.02
```

A response status of 400 or above counts as a failure, and the process exits
with status 1 when any request fails, so the harness can gate a pipeline.

The generator calls the router in-process rather than over a socket. The
reported latency is therefore handler plus database latency and excludes
kernel networking, TLS, and HTTP wire parsing; treat it as an application
performance signal, not as end-to-end client latency. Run it against a
disposable database, never production.

The engine is also usable as a library: `beamrs::loadtest::run_load_test`
accepts any `axum::Router` and a `LoadProfile`.

## Validation

```sh
./scripts/validate-qrspi.sh docs/qrspi/2026-10-06-beamrs-parity
./scripts/validate-qrspi.sh docs/qrspi/2026-10-08-load-testing
cd beamrs
cargo fmt --all -- --check
cargo clippy --all-targets --features ssr -- -D warnings
cargo clippy --lib --no-default-features --features hydrate \
  --target wasm32-unknown-unknown -- -D warnings
cargo test --all-targets
cargo leptos build --release
```

CI also starts the application against PostgreSQL and checks the health route.

## Security model

BeamRS intentionally preserves Beam's unauthenticated identity model. A
username is a display identity, not proof of ownership, and anyone with API
access can act as any known user ID. Do not deploy this version where identity,
authorization, privacy, or abuse resistance is required without adding an
authentication and authorization layer.

User-controlled text is validated, rendered as text, and escaped in generated
markup. Database constraints reinforce application validation. This repository
does not copy Beam's bundled assets or source verbatim.

## QRSPI+

The implementation followed the repository's QRSPI+ workflow. The approved
run, research, design, tasks, phase gates, and verification records live in
[`docs/qrspi/2026-10-06-beamrs-parity/`](docs/qrspi/2026-10-06-beamrs-parity/).
The Leptos 0.8.21 compatibility migration is recorded separately in
[`docs/qrspi/2026-10-06-leptos-0-8-21/`](docs/qrspi/2026-10-06-leptos-0-8-21/),
and the load-testing capability in
[`docs/qrspi/2026-10-08-load-testing/`](docs/qrspi/2026-10-08-load-testing/).

BeamRS is inspired by Dedac/Beam and is an independent Rust implementation.
