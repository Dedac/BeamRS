---
status: approved
phase: parallelize
---

# Parallelization

Execution mode: **sequential**.

All four tasks touch one package and a single new module whose public API the
binary, documentation, and validation all depend on. Task 2 consumes Task 1's
interface, Task 3 documents Task 2's flags, and Task 4 validates the whole
slice, so there is no independent wave to schedule.

```yaml
waves:
  - tasks: [task-01-engine]
    base: phase-start
  - tasks: [task-02-cli]
    base: task-01-engine
  - tasks: [task-03-docs]
    base: task-02-cli
  - tasks: [task-04-validation]
    base: task-03-docs
```
