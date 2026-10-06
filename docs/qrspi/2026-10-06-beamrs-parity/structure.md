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
│   ├── config.rs
│   ├── domain.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── repository.rs
│   └── server.rs
└── tests/
    └── parity.rs
```

This structure keeps the domain, persistence, and server boundaries small and reviewable.
