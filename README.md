# QRSPI Methodology

A portable, repository-first workflow for reliable AI-assisted software delivery.

QRSPI+ turns an ambiguous request into a sequence of small, reviewable artifacts:

```text
Goals -> Questions -> Research -> Design -> Phasing -> Structure -> Plan
      -> Parallelize -> Implement -> Integrate -> Test -> Replan
```

Each phase has a narrow purpose, writes its output to `docs/qrspi/<run>/`, and
requires explicit approval before downstream work begins. The workflow is
designed for Claude Code, Copilot CLI, Codex, and other agents that can read
and write repository files.

## Use it in a project

1. Copy `templates/qrspi/` and `scripts/validate-qrspi.sh` into the target
   repository.
2. Create a run directory:

   ```sh
   mkdir -p docs/qrspi/2026-10-06-example
   cp templates/qrspi/config.md \
      docs/qrspi/2026-10-06-example/config.md
   ```

3. Start with `goals.md`, then advance only after the current artifact is
   approved.
4. Validate the run at every gate:

   ```sh
   ./scripts/validate-qrspi.sh docs/qrspi/2026-10-06-example
   ```

## Repository layout

```text
docs/qrspi/<run>/
├── config.md
├── goals.md
├── questions.md
├── research/
│   ├── q01.md
│   └── summary.md
├── design.md
├── phasing.md
├── roadmap.md
├── structure.md
├── plan.md
├── tasks/
├── parallelization.md
└── reviews/
```

The templates are intentionally tool-neutral. They define the handoff contract;
an agent harness or human can perform each phase.

## Operating rules

- Research questions are neutral and must not prescribe the solution.
- Research records facts and evidence before recommendations.
- Design and structure are reviewed before detailed planning.
- Work is split into vertical slices with explicit validation.
- Production code follows a failing-test-first loop where practical.
- Parallel work requires a dependency and file-overlap analysis.
- CI, integration review, and acceptance tests are separate gates.
- Fixes return through the same implementation and review path.
- Architectural changes loop back to the earliest affected artifact.
- A quick-fix route is appropriate for small, localized changes.

## Publishing

This workspace contains the implementation scaffold and can be initialized as
the future `Dedac/qrspi-methodology` repository:

```sh
git init
git add .
git commit -m "feat: add portable QRSPI methodology"
git branch -M main
git remote add origin https://github.com/Dedac/qrspi-methodology.git
git push -u origin main
```

Create the empty GitHub repository before running the final `git push`.

