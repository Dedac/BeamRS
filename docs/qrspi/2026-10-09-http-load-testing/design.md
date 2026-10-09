---
status: approved
phase: design
---

# Design

Add a `Transport` enum to the engine with `InProcess(Router)` and
`Http(HttpTarget)` variants. A new `run_load_test_with` function takes a
`&Transport`, and the existing `run_load_test(router, profile)` becomes a
wrapper around it, so library callers are unaffected.

## Components

- `HttpTarget::parse` accepts only `http://host[:port]` with no path, query,
  or fragment. HTTPS is rejected explicitly because there is no TLS
  connector.
- `serve_loopback(router)` binds `127.0.0.1:0`, spawns `axum::serve`, and
  returns the target with its `JoinHandle`. The caller aborts the handle when
  the run ends.
- An internal `Sender` holds either the router or one shared hyper-util
  client. The client uses `TCP_NODELAY` and `pool_max_idle_per_host =
  concurrency`, so each worker can keep its own keep-alive connection.
- Request URIs are `"{base_url}{path}"` for HTTP and `path` for in-process.
  Before any worker starts, every spec is validated against the chosen
  prefix.
- `LoadReport` gains a `transport` field (`in-process` or `http`), which
  appears in both the text and JSON output.
- Undeliverable requests are recorded with `transport_error = true`, counted
  as failures, and keyed `transport-error`.
- `HttpTarget` carries a request deadline, defaulting to 30 seconds. Its
  configurable timeout wraps both waiting for response headers and draining
  the complete response body. Expiration is recorded as a `transport-error`.
  In-process transport remains without this HTTP-specific deadline.

## CLI

- `--transport in-process|http` defaults to `in-process`.
- `--target URL` implies `http` and skips the database connection.
- Combining `--target` with an explicit `--transport in-process` is a usage
  error.
- `--http-timeout SEC` configures the whole-request deadline for HTTP modes
  and defaults to 30 seconds. The library can override it with
  `HttpTarget::with_timeout`. Passing the flag without HTTP mode is rejected
  rather than silently ignored.

## Acceptance

- G1 and G2: loopback and external HTTP runs return correct status counts.
  This is covered by engine tests and a release run against the disposable
  database.
- G3: all existing engine and CLI tests pass unchanged, and the in-process
  label is asserted.
- G4: a test against a closed port reports every request as
  `transport-error`.
- G5: slow headers and a response body that never completes both expire
  within the configured deadline and report `transport-error`; the CLI parses
  positive `--http-timeout` values, rejects zero, and rejects use without an
  HTTP transport.
