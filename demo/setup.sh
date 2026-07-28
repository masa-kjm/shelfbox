#!/usr/bin/env bash
set -euo pipefail

demo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
workspace="${demo_dir}/workspace"
web_repo="${workspace}/sample-repo"

if [[ "${1:-}" == "--reset" ]]; then
    # workspace is derived from this script's directory and is never supplied
    # by the caller, so reset cannot target a real repository or store.
    rm -rf -- "${workspace}"
elif [[ $# -ne 0 ]]; then
    printf 'usage: %s [--reset]\n' "$0" >&2
    exit 2
fi

if [[ -e "${workspace}" ]]; then
    printf 'Demo workspace already exists. Run %s --reset to recreate it.\n' "$0" >&2
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

init_repo "${web_repo}" 'Sample app'

# This file intentionally remains untracked so it is eligible for shelving in
# the recording.
printf 'API_TOKEN=demo-token\n' > "${web_repo}/.env"

printf 'Demo repository: %s\n' "${web_repo}"
