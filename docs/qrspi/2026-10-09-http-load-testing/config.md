---
status: approved
pipeline: full
run: 2026-10-09-http-load-testing
---

# QRSPI run configuration

## Route

```yaml
route:
  - goals
  - questions
  - research
  - design
  - phasing
  - structure
  - plan
  - parallelize
  - implement
  - integrate
  - test
  - replan
```

## Scope

Extend the existing `loadtest` harness (run `2026-10-08-load-testing`) so the
same scenarios can be sent over real HTTP/1.1 sockets, either to a loopback
server hosted by the harness or to an already running BeamRS server. The
in-process transport stays the default, and application routes, schema, and
UI are unchanged. A reviewer follow-up adds a configurable deadline for HTTP
requests and response-body reads; in-process behavior remains unchanged.

## Approval record

The user asked to "Make changes to run these over Http" after an in-process
10k req/s run. The session ran in autopilot with no interactive reviewer and
self-approved the artifacts it produced. The Integrate and Replan stages in
the configured route were skipped during the original work; neither stage was
performed, approved, or documented then. After a reviewer identified the
missing records, retrospective assessments were added in
`reviews/integration-review.md` and `reviews/phase-review.md`. They describe
the later review only and do not represent the skipped stages as completed.
The user later requested that the remaining HTTP-timeout review finding be
addressed and selected a 30-second default. The implementation and acceptance
records were amended accordingly; this does not change the historical status
of the skipped Integrate and Replan stages.
