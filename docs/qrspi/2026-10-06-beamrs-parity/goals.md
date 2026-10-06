---
status: approved
phase: goals
---

# Goals

## Purpose

Implement BeamRS as a feature-parity reimplementation of the public Beam social prototype in Rust, preserving the core user, frequency, ray, and prism interactions while improving safety and deployability.

## Constraints

- Preserve the original Beam domain and web behavior from the public repo.
- Use a PostgreSQL-backed stack with Axum and Leptos.
- Avoid unsafe GET mutation semantics; any state change must be explicit and server-side validated.
- Keep the app demo-ready with migrations and seed data.

## Goal G-01: Domain parity

### Problem

Beam's core Model is small but important: users, frequencies, rays, and prisms must behave consistently and persistently.

### Why we care

The correct identity and feed semantics underpin the user experience, validation, and API stability.

### What we know so far

The original app uses anonymous usernames (`AnonN`), a list of frequencies, a ray feed per frequency, and prism toggling keyed by user and ray.

## Goal G-02: API and UI parity

### Problem

The original Beam app includes home, settings, feed, user profile, and prism interactions but the public repo only represents a narrow .NET implementation.

### Why we care

The user-facing feature set must be reproducible without copying the original app's copyrighted assets or source verbatim.

### What we know so far

Key behavior includes frequency creation, ray creation, prism toggle counts, profile views, and a settings screen with a beam animation.
