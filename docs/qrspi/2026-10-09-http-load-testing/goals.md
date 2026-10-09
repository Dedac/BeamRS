---
status: approved
phase: goals
---

# Goals

G1. Run the existing `health`, `read-mix`, and `write-mix` scenarios over
real HTTP/1.1 on TCP, so the measured latency includes connection handling,
HTTP parsing, and the server's middleware.

G2. Support two HTTP targets. The harness can host the real router on an
ephemeral loopback port, which needs no separate process, or it can load an
already running server given its URL.

G3. Keep the in-process transport as the default, with unchanged behavior,
so existing commands and results stay comparable.

G4. Report undeliverable requests (refused or reset connections) as failures
that are distinct from HTTP error statuses.

G5. Bound HTTP request execution through complete response-body reading with a
configurable deadline, defaulting to 30 seconds; report deadline expiration as
a `transport-error`.

## Constraints

- Use crates that are already in the lockfile. Do not upgrade any existing
  dependency version.
- No application route, schema, or UI changes.
- Use a disposable local database only.
- The request deadline applies only to HTTP transports; in-process behavior
  remains unchanged.

## Non-goals

- HTTPS/TLS, HTTP/2, open-loop fixed-rate pacing, and distributed load
  generation.
