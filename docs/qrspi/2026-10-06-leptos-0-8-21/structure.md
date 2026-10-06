---
status: approved
phase: structure
---

# Structure

The migration touches the existing package surfaces only:

- `beamrs/Cargo.toml` and `beamrs/Cargo.lock` for dependency resolution.
- `beamrs/src/{app,lib,server}.rs` if compiler diagnostics require API
  adjustments.
- `.github/workflows/beamrs-ci.yml`, `beamrs/Dockerfile`, and `README.md` if
  their Leptos tooling versions or commands must track the upgrade.

No new runtime layer or alternate frontend is introduced.
