#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: linked-issues.sh [--branch <branch>] [--title <title>]

Extract literal #N references from a Shelfbox branch name and proposed commit or pull-request title.
When at least one reference exists, print its GitHub issue JSON using gh.
USAGE
}

branch="$(git branch --show-current 2>/dev/null || true)"
title=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --branch)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      branch="$2"
      shift 2
      ;;
    --title)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      title="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

mapfile -t issues < <(
  printf '%s\n%s\n' "$branch" "$title" \
    | grep -oE '#[0-9]+' \
    | tr -d '#' \
    | sort -un \
    || true
)

if [[ ${#issues[@]} -eq 0 ]]; then
  printf 'No #N issue reference found in the branch name or title.\n'
  exit 0
fi

if ! command -v gh > /dev/null; then
  printf 'error: gh is required to inspect linked issues.\n' >&2
  exit 3
fi

for issue in "${issues[@]}"; do
  gh issue view "$issue" --json number,title,body,labels,state,url
done
