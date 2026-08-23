#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: check-release-metadata.sh vX.Y.Z [--require-main]

Check the workspace version, exact shelfbox-core dependency in Cargo.toml, first release CHANGELOG heading, package contents, and locked Cargo resolution.
--require-main also requires HEAD to exactly match the locally fetched origin/main.
USAGE
}

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_main=0

if [[ "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage >&2
  exit 2
fi

tag="$1"
if [[ $# -eq 2 ]]; then
  [[ "$2" == "--require-main" ]] || {
    usage >&2
    exit 2
  }
  require_main=1
fi

version="${tag#v}"
[[ "$tag" == "v${version}" && "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  || fail "release tag must use the vX.Y.Z format"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel)" \
  || fail "run this script from a Shelfbox Git checkout"
cd "$repo_root"

manifest_version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)"
[[ -n "$manifest_version" ]] || fail "workspace version is missing from Cargo.toml"
[[ "$manifest_version" == "$version" ]] \
  || fail "Cargo.toml version is ${manifest_version}, expected ${version}"

core_dependency_version="$(sed -n 's/^shelfbox-core = { path = "crates\/shelfbox-core", version = "=\([^"]*\)" }$/\1/p' Cargo.toml)"
[[ -n "$core_dependency_version" ]] || fail "exact shelfbox-core workspace dependency version is missing from Cargo.toml"
[[ "$core_dependency_version" == "$version" ]] \
  || fail "shelfbox-core dependency version is ${core_dependency_version}, expected ${version}"

first_release_heading="$(awk '/^## v[0-9]+\.[0-9]+\.[0-9]+$/ { print; exit }' CHANGELOG.md)"
[[ "$first_release_heading" == "## ${tag}" ]] \
  || fail "CHANGELOG.md must start its release entries with: ## ${tag}"

cargo metadata --locked --no-deps --format-version 1 > /dev/null \
  || fail "Cargo.lock is stale or Cargo metadata could not be resolved with --locked"

cargo package --workspace --allow-dirty --locked --list > /dev/null \
  || fail "package manifests or publish contents are invalid"

if [[ "$require_main" -eq 1 ]]; then
  git rev-parse --verify --quiet origin/main > /dev/null \
    || fail "origin/main is unavailable; fetch it before using --require-main"
  head_commit="$(git rev-parse HEAD)"
  origin_main_commit="$(git rev-parse origin/main)"
  [[ "$head_commit" == "$origin_main_commit" ]] \
    || fail "HEAD does not match the locally fetched origin/main"
fi

printf 'ok: %s matches Cargo.toml, CHANGELOG.md, package contents, and Cargo.lock\n' "$tag"
