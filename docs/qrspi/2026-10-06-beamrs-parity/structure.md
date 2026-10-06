---
status: approved
phase: structure
---

# Structure

```text
beamrs/
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── .env.example
├── migrations/
│   ├── 001_initial.sql
│   └── 002_seed_demo.sql
├── src/
│   ├── app.rs
│   ├── client.rs
│   ├── config.rs
│   ├── domain.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── repository.rs
│   └── server.rs
├── public/
└── static/
    └── style.css
```

`app.rs` is shared by SSR and hydration. `client.rs` contains the typed
browser-side REST/storage boundary and compiles real implementations only for
the hydration target. cargo-leptos writes generated JavaScript, WASM, and CSS
to `target/site/pkg`; generated output is not committed.
