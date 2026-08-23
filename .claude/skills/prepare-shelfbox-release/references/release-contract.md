# Shelfbox Release Contract

The authoritative workflow is [`CONTRIBUTING.md`](../../../../CONTRIBUTING.md) and [`.github/workflows/release.yml`](../../../../.github/workflows/release.yml). This reference turns those requirements into a release-review checklist; it does not replace them.

## Release candidate contents

For target tag `vX.Y.Z`, the release pull request must contain all of the following:

- Root `[workspace.package].version` in `Cargo.toml` is `X.Y.Z`.
- The workspace `shelfbox-core` dependency is the matching exact version `=X.Y.Z`.
- `Cargo.lock` is regenerated without an unexplained dependency refresh.
- `CHANGELOG.md` starts its release entries with the exact heading `## vX.Y.Z`.
- README and user documentation change only when installation, upgrade, compatibility, or user-visible guidance changes.

Workspace crates inherit the workspace package version. Do not introduce crate-specific versions or allow the CLI and `shelfbox-core` dependency versions to drift.

Do not rewrite historical version references in specifications, architecture baselines, or older changelog entries merely because they mention a previous release.

## Scope and publication prerequisites

Keep a release pull request to release metadata and necessary documentation. Put source behavior changes in separately reviewed pull requests.

The tag workflow publishes `shelfbox-core`, waits for it to be available, then publishes `shelfbox`, creates a GitHub Release, and dispatches the Homebrew tap update. Before the tag is pushed, the repository owner must have confirmed the required Actions secrets are available:

- `CARGO_REGISTRY_TOKEN`
- `HOMEBREW_TAP_TOKEN`

Do not inspect, create, replace, or disclose secrets. If their availability is not confirmed, report the release prerequisite instead of assuming it.
