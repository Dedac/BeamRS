---
status: approved
---

# Goals

- Upgrade the application from Leptos 0.6 to exactly Leptos 0.8.21.
- Align the Leptos ecosystem, Axum integration, and cargo-leptos tooling.
- Preserve the existing SSR, hydration, REST API, PostgreSQL, and browser flows.
- Keep the change limited to migration work and exclude unrelated security work.
- Validate native and WASM builds, tests, release assets, container runtime, and
  the required browser paths.

## Success criteria

The application compiles without warnings for SSR and hydration, all tests pass,
the release build emits usable JavaScript and WASM, Docker runs against
PostgreSQL, and the home, create-frequency, frequency, and settings flows work
in a browser.
