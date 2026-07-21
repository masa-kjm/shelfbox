#!/usr/bin/env bash
set -euo pipefail

demo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
workspace="${demo_dir}/workspace"
repo="${workspace}/repo"
store="${workspace}/store"

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

mkdir -p "${repo}" "${store}"
git -C "${repo}" init --quiet --initial-branch=main
git -C "${repo}" config user.name 'shelfbox demo'
git -C "${repo}" config user.email 'demo@example.invalid'

printf '# shelfbox demo\n' > "${repo}/README.md"
git -C "${repo}" add README.md
git -C "${repo}" commit --quiet --message 'Initial commit'

# This file intentionally remains untracked so it is eligible for shelving.
printf 'API_TOKEN=demo-token\n' > "${repo}/.env"

printf 'Demo repository: %s\nDemo store: %s\n' "${repo}" "${store}"
