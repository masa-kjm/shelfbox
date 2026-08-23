#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
checker_source="$script_dir/check-release-metadata.sh"
fixture_root="$(mktemp -d)"
fixture_repo="$fixture_root/repo"
fake_bin="$fixture_root/bin"

cleanup() {
  rm -rf -- "$fixture_root"
}

trap cleanup EXIT

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

write_changelog() {
  printf '%s\n' "$@" > "$fixture_repo/CHANGELOG.md"
}

run_checker() {
  (
    cd "$fixture_repo"
    PATH="$fake_bin:$PATH" bash "$fixture_repo/.claude/skills/prepare-shelfbox-release/scripts/check-release-metadata.sh" "$@"
  )
}

expect_failure() {
  local expected_message="$1"
  shift

  local output
  if output="$("$@" 2>&1)"; then
    fail "expected failure: $*"
  fi
  [[ "$output" == *"$expected_message"* ]] \
    || fail "failure output did not include '$expected_message': $output"
}

mkdir -p "$fixture_repo/.claude/skills/prepare-shelfbox-release/scripts" "$fake_bin"
cp "$checker_source" "$fixture_repo/.claude/skills/prepare-shelfbox-release/scripts/check-release-metadata.sh"
chmod +x "$fixture_repo/.claude/skills/prepare-shelfbox-release/scripts/check-release-metadata.sh"

cat > "$fixture_repo/Cargo.toml" <<'TOML'
[workspace]
members = []

[workspace.package]
version = "1.2.3"

[workspace.dependencies]
shelfbox-core = { path = "crates/shelfbox-core", version = "=1.2.3" }
TOML

write_changelog '# Changelog' '' '## v1.2.3' '' '- Test release.'

cat > "$fake_bin/cargo" <<'SH'
#!/usr/bin/env bash
case "${1:-}" in
  metadata | package) exit 0 ;;
  *) exit 1 ;;
esac
SH
chmod +x "$fake_bin/cargo"

git -C "$fixture_repo" init --initial-branch=main --quiet
git -C "$fixture_repo" config user.email "release-checker-test@example.invalid"
git -C "$fixture_repo" config user.name "Release Checker Test"
git -C "$fixture_repo" add .
git -C "$fixture_repo" commit --quiet -m 'test fixture'
git -C "$fixture_repo" update-ref refs/remotes/origin/main HEAD

run_checker v1.2.3
run_checker v1.2.3 --require-main

write_changelog '# Changelog' '' '## v9.9.9' '' '- Newer entry.' '' '## v1.2.3' '' '- Test release.'
expect_failure 'CHANGELOG.md must start its release entries with: ## v1.2.3' run_checker v1.2.3

write_changelog '# Changelog' '' '## v1.2.3' '' '- Test release.'
old_head="$(git -C "$fixture_repo" rev-parse HEAD)"
git -C "$fixture_repo" commit --quiet --allow-empty -m 'advance main'
git -C "$fixture_repo" update-ref refs/remotes/origin/main HEAD
git -C "$fixture_repo" checkout --quiet --detach "$old_head"
expect_failure 'HEAD does not match the locally fetched origin/main' run_checker v1.2.3 --require-main

git -C "$fixture_repo" update-ref -d refs/remotes/origin/main
expect_failure 'origin/main is unavailable' run_checker v1.2.3 --require-main

printf 'ok: release metadata checker contract tests passed\n'
