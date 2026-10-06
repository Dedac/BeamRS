# QRSPI+ operating model

## Purpose

QRSPI+ separates alignment, execution, and verification so an AI agent does
not make hidden architectural decisions while writing code. Each stage has a
single responsibility and a durable artifact.

## Phase contracts

| Phase | Input | Output | Gate |
|---|---|---|---|
| Goals | request and conversation | `goals.md` | intent and scope approved |
| Questions | approved goals | `questions.md` | research is complete enough |
| Research | questions only | `research/` | facts and evidence reviewed |
| Design | goals and research | `design.md` | approach selected |
| Phasing | design | `phasing.md`, `roadmap.md` | slices and boundaries approved |
| Structure | design and phasing | `structure.md` | files and interfaces mapped |
| Plan | prior artifacts | `plan.md`, `tasks/` | tasks are executable |
| Parallelize | plan and tasks | `parallelization.md` | dependency graph approved |
| Implement | task specs | code and task reviews | tests pass for each task |
| Integrate | task branches | merged branch and review | cross-task behavior verified |
| Test | merged branch and criteria | acceptance review | criteria exercised |
| Replan | completed phase | approved amendments | next phase is grounded |

## Context boundaries

The artifact is the handoff channel. A phase should receive only the artifacts
listed in its contract, plus repository context needed to answer its question.
Starting a fresh context at each boundary prevents stale assumptions and keeps
the active prompt small.

## Review allocation

Spend the most human attention on goals, design, structure, and shipped code.
Spot-check detailed plans after those higher-leverage decisions are correct.
Use specialized reviews for security, silent failures, specification
compliance, test coverage, and integration boundaries.

## Routing changes

- **Clarifying:** update the current artifact and re-approve it.
- **Additive:** add tasks within the current slice and repeat the affected gate.
- **Architectural:** return to Design or an earlier phase.
- **Unknown scope:** treat as architectural until proven otherwise.

