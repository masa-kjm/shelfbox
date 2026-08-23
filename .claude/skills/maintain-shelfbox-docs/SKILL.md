---
name: maintain-shelfbox-docs
description: Maintain Shelfbox documentation when changing user behavior, workflows, command reference, specifications, architecture, or documentation navigation.
---

# Maintain Shelfbox Documentation

Keep Shelfbox documentation accurate, navigable, and aligned with its behavioral contracts. Use this skill for documentation changes in `README.md` or `docs/`, including documentation that accompanies a code change. Do not use it for a code-only change with no documentation impact, for a behavior-change review, or for release preparation; use the relevant dedicated skill in those cases.

## Establish the source of truth

1. Read `docs/index.md`, the target page, and every directly relevant page it links to before editing.
2. Identify the claim being changed and its authority:

   | Claim | Authority |
   | --- | --- |
   | Public behavior, safety, ownership, recovery, persistence, compatibility | The task or linked issue acceptance criteria and the applicable `docs/spec/` contract |
   | Command spelling, flags, output, and routine errors | The applicable `docs/reference/` page, checked against the CLI/API and compatibility tests when the contract is changing |
   | User journey and recovery procedure | The applicable guide, linked command reference, and any relevant specification |
   | Module boundaries, data flow, and design rationale | The applicable architecture page, checked against the current code |
   | Installation and distribution | The installation guide, checked against the current installer, package metadata, or release workflow |

3. Do not infer an intentional behavior change from implementation details, examples, or comments. If the authorities conflict or leave a safety-, ownership-, persistence-, or compatibility-sensitive decision unclear, report the discrepancy before choosing an outcome.

`docs/spec/` defines normative behavior. A command reference defines its interface, but must not silently weaken or redefine a specification. User guides may summarize the contract but must link to it instead of duplicating it.

## Put information in its canonical home

| Location | Owns | Does not own |
| --- | --- | --- |
| `README.md` | First impression, essential examples, and links into the documentation | Exhaustive command semantics or normative contracts |
| `docs/index.md` | The complete documentation map and reading routes | Duplicated page content |
| `docs/guide/` | Goal-oriented workflows, preconditions, expected results, and recovery routes | Full option tables or an independent behavior contract |
| `docs/reference/` | Exact command syntax, flags, output, and user-observable routine errors | Safety, ownership, or recovery rules already defined by a specification |
| `docs/spec/` | Testable invariants, state transitions, authority rules, failure aftermath, and compatibility guarantees | Tutorial prose or implementation structure |
| `docs/architecture/` | Current module responsibilities, dependency direction, data flow, and design rationale | A user-facing command manual or a substitute for a normative specification |

When creating, splitting, renaming, or removing a page, update `docs/index.md`. Update `README.md` only when its landing-page promise, essential example, or direct navigation is affected. Prefer changing the canonical page and linking to it over copying a rule into several pages.

## Make the documentation change

- Keep examples executable as written. Check command names, flags, defaults, confirmation requirements, output claims, and platform qualifications against their authority.
- For a mutating command or workflow, state the relevant scope, dry-run behavior, confirmation requirement, and recovery or safety constraint. Link to the specification rather than restating a long invariant.
- For a changed user-visible contract, update the specification, command reference, guide, README, and compatibility fixture only where each has a real responsibility. A source-code change alone is not evidence that every document needs editing.
- For an architecture change, update the module map or other architecture page only after confirming the resulting dependency direction and public/private boundaries in code.
- Use durable, descriptive headings and relative repository links. Do not leave links to a moved page, a renamed heading, or an obsolete command.
- Keep documentation in English and follow the repository's established Markdown style.

## Validate the result

1. Inspect the rendered-reading path: the changed page, its inbound links from `README.md` and `docs/index.md`, and any guide or reference page that summarizes the altered contract.
2. When Markdown links or headings change, run the local-link checker from the invoked skill directory:

   ```sh
   python3 <skill-dir>/scripts/check-local-links.py README.md docs
   ```

   It validates ordinary repository-local inline links and heading fragments; inspect unsupported Markdown constructs manually.
3. Run `git diff --check`. Run any focused compatibility, CLI, or documentation checks needed to substantiate changed examples or behavioral claims.
4. Do not report a validation command as passed when it was not run. State any intentionally deferred navigation or validation risk.

## Handoff

State the canonical page changed, the source used to verify each behavior claim, dependent README/index/guide/reference/spec pages updated, and the validation performed. Call out unresolved authority conflicts rather than presenting a guessed contract as settled.
