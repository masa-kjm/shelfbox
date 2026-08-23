---
name: prepare-shelfbox-release
description: Prepare or validate a Shelfbox release pull request and its matching vX.Y.Z tag. Use for release metadata, release candidates, tag readiness, or release-workflow follow-up; not for ordinary feature changes.
---

# Prepare Shelfbox Release

Prepare a focused, reviewable release. Do not infer the target version or perform a merge, tag, push, publication, or other external action unless the user explicitly requests it.

## Establish the requested release stage

1. Determine whether the request is to prepare a release pull request, validate a candidate, or perform a post-merge tag step.
2. Read `CONTRIBUTING.md`, the root `Cargo.toml`, the top of `CHANGELOG.md`, and `.github/workflows/release.yml` before changing release metadata.
3. Require a stable target tag in the exact `vX.Y.Z` format. The release branch convention is `release/vX.Y.Z`.
4. Inspect the worktree and preserve unrelated changes. Keep source behavior changes out of a release-only pull request.

## Prepare the release candidate

For the required files, version relationship, changelog, documentation scope, and workflow secrets, read [the release contract](references/release-contract.md).

Use the release metadata checker adjacent to this skill's `SKILL.md`; do not assume that the skill was installed under a particular directory. Read [release validation](references/release-validation.md) before running it or interpreting its result.

Ask for `$review-shelfbox-change` only when the release candidate also changes behavior or release automation. A metadata-only release is reviewed against the release contract and the diff.

## Complete only with explicit authority

After the release pull request is reviewed and its required checks pass, merging, tagging, and pushing each require explicit user authorization. Before tagging, use the post-merge validation described in [release validation](references/release-validation.md).

Do not run `cargo publish` manually for the normal workflow. A matching pushed tag starts the repository release workflow; confirm that workflow's outcomes before reporting a release as available.

## Handoff

Report the target version, changed release files, validation results, and any checks that could not run. Distinguish a prepared candidate, a pushed tag, and a completed published release.
