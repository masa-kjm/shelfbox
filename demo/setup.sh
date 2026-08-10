#!/usr/bin/env bash
set -euo pipefail

demo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="${demo_dir}/sample-repo"

if [[ "${1:-}" == "--reset" ]]; then
    # The demo repository is created under the demo directory and is not
    # expected to be reused across runs.
    rm -rf -- "${repo_root}"
elif [[ $# -ne 0 ]]; then
    printf 'usage: %s [--reset]\n' "$0" >&2
    exit 2
fi

if [[ -e "${repo_root}" ]]; then
    printf 'Demo repository already exists. Run %s --reset to recreate it.\n' "$0" >&2
    exit 1
fi

init_repo() {
    local repo="$1"
    local title="$2"

    mkdir -p "${repo}"
    git -C "${repo}" init --quiet --initial-branch=main
    git -C "${repo}" config user.name 'shelfbox demo'
    git -C "${repo}" config user.email 'demo@example.invalid'
    printf '# %s\n' "${title}" > "${repo}/README.md"
    git -C "${repo}" add README.md
    git -C "${repo}" commit --quiet --message 'Initial commit'
}

init_repo "${repo_root}" 'Sample app'

printf 'Demo repository: %s\n' "${repo_root}"
