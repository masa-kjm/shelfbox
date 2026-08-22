# Contributing to shelfbox

This document defines the development workflow for shelfbox. Keep `main` releasable: every merged change must be reviewed and pass the required CI checks.

## Issues

Create an issue before implementing a user-visible behavior change or bug fix.
Write issues so that a reviewer can determine correctness without knowing the intended implementation.

Use this structure:

```markdown
## Problem

Describe the observed failure and its impact.

## Reproduction

1. List the smallest reliable steps.

## Expected behavior

- Describe the user-visible result.

## Acceptance criteria

- State independently testable requirements, including safety and no-change guarantees where relevant.
```

State filesystem scope, dry-run behavior, failure aftermath, and platform expectations explicitly for mutations. Link related issues or specifications, and identify intentional non-goals when they narrow the change.

## Branches

Start a short-lived branch from the current `main`. Use lowercase kebab-case names and choose the prefix that describes the change:

| Purpose | Branch pattern | Example |
| --- | --- | --- |
| User-visible feature | `feat/<description>` | `feat/repo-reclaim-output` |
| Bug fix | `fix/<description>` | `fix/repair-missing-parents` |
| Documentation | `docs/<description>` | `docs/copy-mode-recovery` |
| Refactoring | `refactor/<description>` | `refactor/materializer-plan` |
| Tests | `test/<description>` | `test/repair-path-escape` |
| Tooling or maintenance | `chore/<description>` | `chore/update-rust-toolchain` |
| Release preparation | `release/vX.Y.Z` | `release/v0.9.2` |

Keep a branch focused on one issue or release. Rebase or merge the latest `main` as needed before opening a pull request.

## Commits

Use Conventional Commits with an optional scope:

```text
<type>(<scope>): <imperative summary>
```

Use `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, or `ci` as the type.
Keep the summary concise, imperative, and specific. Use a scope when it helps locate the behavior, such as `repair`, `repo`, `item`, `fs`, or `ci`.
Use scopes for responsibility or behavior areas, not issue numbers; link the issue in the pull request and, when useful, in the commit body.

Examples:

```text
fix(repair): recreate missing materialization parents
test(repair): cover symlink parent escape
ci: require locked Cargo resolution
```

Make each commit buildable when practical and avoid unrelated formatting or drive-by changes. Do not mix a release version bump with unrelated behavior changes.

## Pull requests

Open every pull request against `main` and link its issue (for example, `Closes #123`). Describe the behavior change, the safety properties, and tests run; call out platform-specific limitations or checks that could not run.
Do not push directly to `main`.

For behavior changes, describe how the pull request satisfies the relevant acceptance criteria, including the affected behavior, safety properties, and test evidence. Use a detailed criterion-to-evidence mapping when the change affects filesystem mutations, ownership, persistence, recovery, or compatibility.

Before requesting review:

- Rebase or otherwise reconcile with current `main`.
- Run the relevant formatter, linter, and tests with `--locked` where Cargo is involved.
- Update user documentation and compatibility fixtures when behavior or output changes.
- Ensure the PR meets every issue acceptance criterion with tests or clear reviewable evidence.

Reviewers should use the repository's [`review-shelfbox-change`](.claude/skills/review-shelfbox-change/SKILL.md) skill for behavior changes. It maps acceptance criteria to code and tests and applies the repository's filesystem-safety invariants.

Merge only after an approving review and all required CI checks succeed. Use squash merge so `main` retains one commit per completed pull request.

## Releases

Prepare each release in a dedicated `release/vX.Y.Z` branch and open a release pull request against `main`. The release pull request must contain:

1. The matching `[workspace.package].version` change in `Cargo.toml`.
2. The matching exact `shelfbox-core` workspace dependency version in `Cargo.toml`.
3. The updated generated `Cargo.lock`.
4. A `## vX.Y.Z` section in `CHANGELOG.md`.
5. Any necessary README or documentation changes.

Use the repository's [`prepare-shelfbox-release`](.claude/skills/prepare-shelfbox-release/SKILL.md) skill to make these changes and run the release metadata checks.

After the release pull request is reviewed, passes CI, and is squash-merged to `main`, create and push the matching annotated tag from that commit:

```sh
git switch main
git pull --ff-only origin main
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

Pushing the tag starts the Release workflow.
Verify that its version, changelog, platform build, crates.io publishing, and GitHub Release checks complete before announcing the release.
The repository must have a `CARGO_REGISTRY_TOKEN` Actions secret with permission to publish both crates.
