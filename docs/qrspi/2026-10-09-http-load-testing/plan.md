---
status: approved
phase: plan
phase_start_commit: d44cf49
---

# Plan

## Task 1 — Engine HTTP transport (`tasks/task-01-engine-http.md`)

Add the transport abstraction described in `design.md`.

Test expectations:

- A loopback HTTP run reports exact 200 and 404 counts and the transport
  `http`.
- JSON POST bodies are delivered over HTTP.
- A closed port yields one `transport-error` per request, and all of them
  count as failures.
- Target parsing accepts `http://host:port` with or without a trailing `/`,
  and rejects https URLs, URLs without a scheme, URLs with a path or query,
  and garbage.
- The in-process report is labelled `in-process`, and every existing engine
  test still passes.

## Task 2 — CLI flags (`tasks/task-02-cli-http.md`)

Test expectations:

- The transport defaults to `in-process`, `http` is accepted, and unknown
  values are rejected.
- `--target` implies `http`.
- `--target` combined with `--transport in-process` is rejected.

## Task 3 — Documentation (`tasks/task-03-docs-http.md`)

README flags, transport semantics and caveats, and the validation command.
