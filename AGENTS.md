# Agent instructions

This repository implements the QRSPI+ methodology itself.

## Required workflow

For changes larger than a localized documentation fix, use:

```text
Goals -> Questions -> Research -> Design -> Phasing -> Structure -> Plan
      -> Parallelize -> Implement -> Integrate -> Test -> Replan
```

Keep phase artifacts under `docs/qrspi/<run>/`. Do not silently skip an
approval gate or replace an approved artifact without recording why.

## Quality rules

- Prefer small, reviewable changes.
- State assumptions and unresolved questions explicitly.
- Add tests or validation for behavior changes.
- Use the repository's validator before completing a phase.
- Keep templates tool-neutral and portable POSIX Markdown/shell where possible.

