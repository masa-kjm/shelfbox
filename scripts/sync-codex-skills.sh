#!/usr/bin/env bash
set -euo pipefail

readonly adapter_marker=".shelfbox-codex-adapter"

# Show command-line usage and safety semantics.
usage() {
  cat <<'USAGE'
Usage: sync-codex-skills.sh [--dry-run] [--prune]

Synchronize every skill in .claude/skills into the project-local .agents/skills directory for Codex discovery.

The synchronizer never replaces an unmanaged target skill. Use --prune to remove Shelfbox-managed target skills whose source no longer exists.
USAGE
}

dry_run=false
prune=false

# Parse synchronization options before resolving repository paths.
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      dry_run=true
      shift
      ;;
    --prune)
      prune=true
      shift
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

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repository_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
source_root="$repository_root/.claude/skills"
target_root="$repository_root/.agents/skills"

# Discover only direct child directories that contain a skill definition.
if [[ ! -d "$source_root" ]]; then
  printf 'error: source skill directory not found: %s\n' "$source_root" >&2
  exit 1
fi

source_names=()
for source_dir in "$source_root"/*; do
  [[ -f "$source_dir/SKILL.md" ]] || continue
  source_names+=("$(basename -- "$source_dir")")
done

if [[ ${#source_names[@]} -eq 0 ]]; then
  printf 'error: no skills found in %s\n' "$source_root" >&2
  exit 1
fi

# Refuse to overwrite skills that were not created by this synchronizer.
for skill_name in "${source_names[@]}"; do
  target_dir="$target_root/$skill_name"
  if [[ ( -e "$target_dir" || -L "$target_dir" ) && ! -f "$target_dir/$adapter_marker" ]]; then
    printf 'error: refusing to replace unmanaged target skill: %s\n' "$target_dir" >&2
    exit 1
  fi
done

# Report planned changes without modifying generated skills when requested.
if [[ "$dry_run" == true ]]; then
  for skill_name in "${source_names[@]}"; do
    target_dir="$target_root/$skill_name"
    if [[ -e "$target_dir" || -L "$target_dir" ]]; then
      printf 'Would update %s\n' "$target_dir"
    else
      printf 'Would install %s\n' "$target_dir"
    fi
  done
  if [[ "$prune" == true && -d "$target_root" ]]; then
    for target_dir in "$target_root"/*; do
      [[ -f "$target_dir/$adapter_marker" ]] || continue
      skill_name="$(basename -- "$target_dir")"
      if [[ ! -f "$source_root/$skill_name/SKILL.md" ]]; then
        printf 'Would remove stale managed skill %s\n' "$target_dir"
      fi
    done
  fi
  exit 0
fi

# Stage each copy and atomically replace only Shelfbox-managed target skills.
mkdir -p "$target_root"

stage_dir=""
cleanup() {
  if [[ -n "$stage_dir" && -d "$stage_dir" ]]; then
    rm -rf -- "$stage_dir"
  fi
}

trap cleanup EXIT

for skill_name in "${source_names[@]}"; do
  source_dir="$source_root/$skill_name"
  target_dir="$target_root/$skill_name"
  stage_dir="$(mktemp -d "$target_root/.${skill_name}.XXXXXX")"

  cp -R "$source_dir/." "$stage_dir"
  printf 'source=.claude/skills/%s\n' "$skill_name" > "$stage_dir/$adapter_marker"

  if [[ -e "$target_dir" || -L "$target_dir" ]]; then
    backup_dir="$(mktemp -d "$target_root/.${skill_name}.backup.XXXXXX")"
    rmdir "$backup_dir"
    mv "$target_dir" "$backup_dir"
    if ! mv "$stage_dir" "$target_dir"; then
      printf 'error: failed to update %s; preserved the previous version at %s\n' "$target_dir" "$backup_dir" >&2
      exit 1
    fi
    rm -rf -- "$backup_dir"
    printf 'Updated %s\n' "$target_dir"
  else
    mv "$stage_dir" "$target_dir"
    printf 'Installed %s\n' "$target_dir"
  fi

  stage_dir=""
done

# Optionally remove managed skills whose source definition was deleted.
if [[ "$prune" == true ]]; then
  for target_dir in "$target_root"/*; do
    [[ -f "$target_dir/$adapter_marker" ]] || continue
    skill_name="$(basename -- "$target_dir")"
    [[ -f "$source_root/$skill_name/SKILL.md" ]] && continue
    rm -rf -- "$target_dir"
    printf 'Removed stale managed skill %s\n' "$target_dir"
  done
fi
