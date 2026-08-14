# Changelog

## v0.9.2

- Added release automation to update the Homebrew tap after publishing tagged artifacts.
- `repo sync --from store|repo` now requires `--yes` for overwrite operations in both directions.
- `item repair` now recreates missing parent directories before materialization updates.
- `item restore` now prunes empty store item directories after cleanup.

## v0.9.1

- Added `item materialize --strategy symlink|copy` for explicit, atomic conversion of a healthy item materialization. Conversion never changes manifest identity or ownership, and a diverged copy must be synchronized explicitly first.
- Added `repo sync --from store|repo` and `repo materialize --strategy symlink|copy`. Both validate every attached target before the first write; a runtime failure stops the ordered batch and reports completed items.

## v0.9.0

- Added public Copy materialization mode through `materialization = "copy"`.
- Added explicit `item sync` and directional detached-item relink workflows for regular-copy content changes.
- Added copy-aware status, repair, restore, move, repository repair, and store verification, including durable mutation recovery safeguards.
- `item status` and `repo status` JSON now use status schema version 2:
  copy items expose generic materialization fields and serialize legacy `link_exists` / `link_valid` as `null`.
- `item restore --keep-store` is documented as detach semantics: it retains the observed materialization, canonical store item, manifest entry, and exclude.
- Added local `mutation_durability = "require" | "best-effort"`, defaulting to fail-closed `require`.

## v0.8.0

- Refactored the internal architecture to ensure future maintainability, extensibility, and security.
