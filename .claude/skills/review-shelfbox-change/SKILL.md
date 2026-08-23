---
name: review-shelfbox-change
description: Review Shelfbox behavior changes and pull requests against their acceptance criteria, specifications, architecture, safety guarantees, and compatibility contracts. Use especially for filesystem mutations, ownership, recovery, paths, dry runs, or durable state.
---

# Review Shelfbox Changes

Review evidence, not intent. Do not edit the reviewed implementation unless the task explicitly requests changes.

## Establish the review surface

1. Read `CONTRIBUTING.md`. For a pull request, inspect its issue, description, comments, changed files, and CI status before concluding; use the repository's prescribed GitHub workflow when applicable.
2. Identify the comparison base and inspect the complete diff. Do not assume `main` when the pull request specifies another target.
3. Read the applicable documents linked from `docs/index.md`. Treat the issue acceptance criteria and the applicable `docs/spec/` contract as the behavioral source of truth.
4. List every observable behavior change, including compatible-output, failure-aftermath, and platform-specific changes that are not obvious from the title.
5. Map each acceptance criterion to implementation and test evidence as **verified**, **unverified**, or **violated**.

## Use the relevant review lenses

- For CLI parsing, command handlers, or public core API changes, read [layer boundaries](references/layer-boundaries.md).
- For filesystem writes, persistent state, ownership, recovery, path handling, or dry-run behavior, read [mutation safety](references/mutation-safety.md).
- For every changed or required test, read [testing evidence](references/testing-evidence.md).

Read only the lenses relevant to the review. The repository specifications and architecture documents remain authoritative; these references define the evidence a reviewer should seek.

## Validate and report

Run focused tests and the relevant locked checks when the environment permits. State precisely which commands ran, their results, and why any others could not run.

Report only evidence-backed findings that affect correctness, safety, compatibility, or acceptance criteria. Sort findings by severity and cite a concrete affected location.

```markdown
## Findings

- P1 — Short imperative title ([path](path:line))
  Explain the reachable scenario, the violated contract, and the smallest required correction.

## Acceptance coverage

| Criterion | Status | Implementation and test evidence, or gap |
| --- | --- | --- |
| ... | verified / unverified / violated | ... |

## Validation

- Commands run and their results.
- Commands not run and the concrete reason.
```

If no finding is supported by evidence, say so explicitly. Do not turn a preferred implementation style into a finding when the documented contract is satisfied.
