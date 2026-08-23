# Shelfbox Release Validation

Run validation from the repository root using the toolchain available in the current environment. If that environment requires a shell wrapper or setup step to expose Cargo, use the repository's current agent instructions rather than hard-coding a local setup into this skill.

## Metadata check

Locate the directory that contains the loaded `SKILL.md`, then invoke its adjacent script with the intended tag:

```sh
bash "<skill-directory>/scripts/check-release-metadata.sh" vX.Y.Z
```

The checker verifies the workspace version, exact `shelfbox-core` dependency, changelog heading, locked dependency resolution, and package contents. It is read-only apart from Cargo's normal local cache and build metadata.

For a tag candidate after the release pull request has been merged, run:

```sh
bash "<skill-directory>/scripts/check-release-metadata.sh" vX.Y.Z --require-main
```

`--require-main` proves that `HEAD` and the locally available `origin/main` name the same commit. If that ref is unavailable, report the limitation; do not fetch or alter refs merely to make this check pass unless that action is authorized.

## Candidate checks

Run the applicable commands and report their individual results:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked --bin shelfbox
cargo package --workspace --allow-dirty --locked --list
git diff --check
```

Inspect the complete diff as well. It should contain the release metadata and only justified documentation or release-supporting changes.

## After an authorized tag push

A tag push is not proof that the release completed. Check the Release workflow for tag validation, platform builds, both crates.io publications, GitHub Release creation, and Homebrew tap dispatch. Report any failed or skipped stage precisely.
