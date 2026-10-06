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

- Rust stable
- `wasm32-unknown-unknown`
- `cargo-leptos` 0.2.34
- PostgreSQL 16+, or Docker with Docker Compose

## Run locally

```sh
cd beamrs
cp .env.example .env
docker compose up -d db
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --version 0.2.34 --locked
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

## Validation

```sh
./scripts/validate-qrspi.sh docs/qrspi/2026-10-06-beamrs-parity
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

BeamRS is inspired by Dedac/Beam and is an independent Rust implementation.
