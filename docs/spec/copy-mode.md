# Copy Mode

Copy mode materializes an ordinary file in the repository when a symlink cannot
be created. It is an opt-in fallback: the store remains canonical, and the
default behavior, ownership model, and existing items do not change.

## Enable Copy Mode

Choose copy mode for future materializations:

```sh
shelfbox config set materialization copy
```

`materialization` accepts `symlink` (the default) and `copy`. The setting is a
user-local default, not desired state: it applies when `item add` creates a new
materialization or when repair or relink must recreate a missing one. It never
converts an existing item; use `item materialize` for an explicit conversion.
A mixture of symlink and copy items is healthy.

On Windows, configure reduced durability before the first shelf mutation:

```sh
shelfbox config set mutation_durability best-effort
```

`mutation_durability` is independent from `materialization`. It defaults to
`require`, which fails closed where parent-directory durability is unavailable,
including Windows. `best-effort` is an explicit local opt-in that tolerates
only the unavailable directory-durability capability; it does not suppress
I/O, permission, identity, replacement, Git, or validation errors. Changing
either setting never converts an existing materialization. See the
[configuration reference](../reference/config-commands.md) for all settings.

## Scope

Copy mode never automatically synchronizes or merges repo-side edits, tracks
copy history, materializes non-regular filesystem entries, or persists a
per-item strategy. Use `item sync --from ...` to select a content direction
explicitly, and `item materialize` to convert a healthy materialization.

## Invariants

* `repos/` and `manifest.json` are canonical, while `index.json` is a rebuildable cache.
* The meanings of `RepoId`, `ItemId`, and `ownership_state` do not change.
* The store-side file is always the canonical content.
* A repo-side copy is an editable materialized view; editing it alone does not make it canonical.
* Repo-side changes may be written back to the store only by an operation that explicitly specifies `--from repo`. The normal path is `item sync`; only retrieval of a detached item is handled by `item relink`.
* The copy/symlink distinction is not used to determine GC reachability or ownership.
* A repo-side materialization must not be tracked by Git and must have a Git exclude entry.
* A diverged copy is never implicitly overwritten, moved, or deleted.
* The exclude entry must be added and verified before a regular file is created on the repo side.
* A copy with a missing exclude entry is treated as an integrity error because it can leak content.
* Copy contents are compared byte for byte; metadata does not establish equality.
* A difference between the configured and observed strategy is informational:
  it does not change status severity, exit code, or available operations.

## States and Default Behavior

| observed state | status | repair | sync from store | sync from repo |
| --- | --- | --- | --- | --- |
| missing | error | recreate with configured strategy | reject and suggest repair | reject |
| managed symlink | healthy | no-op | no-op | reject |
| equal regular copy | healthy | no-op | no-op | no-op |
| diverged regular copy | error (`content_diverged`) | report, do not modify | explicitly overwrite repo | explicitly update store |
| unsafe hardlink | error | reject, do not modify | reject | reject |
| unexpected symlink | error | replace with managed symlink only with `--force` | reject | reject |
| unsupported/unreadable | error | reject, do not modify | reject | reject |
| store missing/unreadable | error | reject, do not modify | reject | reject |

Additional conditions:

* Tracked state or a Git query failure is an error regardless of state, and all writes are rejected.
* A symlink with a missing exclude remains a warning, preserving existing behavior.
* A regular copy with a missing exclude is an error because it risks content leakage even when untracked. Reject content/materialization writes except for `repo repair` or a relink phase that repairs the exclude itself.
* An exclude query failure is an error.
* A strategy difference is informational and does not change the severity in this table.

## Command Specifications

### `item add`

`item add` creates a symlink or copy according to the configured strategy.

1. Validate path, regular-file type, Git-untracked state, manifest state, store destination, hardlink state, and path safety.
2. Add the exclude entry and verify it with `has_entry`.
3. Move the source file into the store.
4. Materialize the repo side with the configured strategy.
5. Atomically save the manifest and verify postconditions.

On an ordinary error, remove the materialization created by the operation, return the store file to its original path, restore the manifest snapshot from before the operation, and remove only the exclude entry added by this operation. Rollback may act only when the recorded identity matches, and must not overwrite an entry created by the user or another process.

If rollback itself fails, report both the original and rollback errors and leave
the state for safe recovery on the next run. Directory add preserves the
existing partial-success policy.

### `item status` / `repo status`

Status evaluates facts through the policy evaluator and returns at least:

```rust
enum StatusSeverity {
    Healthy,
    Warning,
    Error,
}

struct ItemStatus {
    status_schema_version: u32,
    path: String,
    configured_strategy: MaterializationStrategy,
    observed_materialization: ObservedMaterialization,
    materialization_exists: bool,
    materialization_valid: bool,
    content_state: CopyContentState,
    store_exists: bool,
    in_exclude: bool,
    not_tracked: bool,
    severity: StatusSeverity,
    issues: Vec<StatusIssue>,
    notes: Vec<StatusNote>,
    ok: bool,
    link_exists: Option<bool>,
    link_valid: Option<bool>,
}
```

`issues` use stable typed codes and may carry remediation and target-path kind where necessary. Information that does not affect integrity, such as a strategy difference, is separated into `notes`. CLI text and JSON present results from the same evaluator.

`materialization_valid` is a structural field indicating whether the entry kind and its relationship to the store are safe. Content divergence and exclude/Git problems belong in `content_state`, `issues`, and `severity`; do not mix them into this field's meaning.

JSON status contract:

* Include `status_schema_version = 2` in every item.
* Preserve the current outer JSON array/repo-report shape.
* The generic contract is `materialization_exists`, `materialization_valid`, and `observed_materialization`.
* `ok` is equivalent to `severity == Healthy`.
* Existing `link_exists` and `link_valid` remain their previous boolean values for symlink items and are `null` for copy items.
* Do not give the legacy booleans generic materialization semantics.
* Preserve existing field values for symlink items and add new fields additively.

Do not add a separate v1 compatibility output. Consumers handling copy items use the schema-v2 generic fields.

Severity and CLI exit code:

* `Healthy` / exit `0`: store, materialization, exclude, and Git state are healthy.
* `Warning` / exit `1`: canonical data exists but a non-destructive repair is
  needed, such as a missing exclude for a managed symlink.
* `Error` / exit `2`: missing materialization, missing store, tracked state, missing exclude for a copy, unsafe hardlink, unexpected entry, path escape, unfinished-operation conflict, and similar states.

For multiple items, use the highest severity as the exit code. `item status` and `repo status` are read-only and perform no repair, reclaim, manifest mutation, or operation recovery.

### `item repair`

Preserve its existing responsibility and do not modify excludes.

* When the exclude is missing or its query fails, do not create a materialization; suggest `repo repair`.
* For a missing materialization, verify exclude, untracked state, and store, then recreate it with the configured strategy.
* Equal copies and valid symlinks are no-ops.
* Report a diverged copy and do not overwrite it. Its status severity remains
  the `content_diverged` error.
* As before, replace a wrong-target symlink with a managed symlink only when `--force` is specified.
* Even with `--force`, do not overwrite regular files, diverged copies, hardlinks, or unsupported entries.
* Do not modify ownership, manifest identity, or exclude state.

### `repo repair`

As an existing repository-integration repair operation, repair materializations only for attached items. Excludes protect attached items and detached items that retain a repo-side materialization.

1. Build the following desired exclude set and apply it to the managed block.
2. Evaluate attached items through materialization policy.
3. Recreate missing materializations with the configured strategy.
4. Replace wrong-target symlinks only with `--force`.
5. Treat equal copies and valid symlinks as no-ops, and report diverged copies
   without modifying them.
6. Update index metadata and identity hints according to existing behavior.

Desired exclude set:

* Attached item: always include it, whether or not a repo entry exists.
* Detached item with a repo entry: always include it.
* Detached item with no repo entry: preserve it when already present in the current managed block, but do not add it.
* Unreachable/orphaned item: do not add it.

Do not overwrite regular-file content with `--force`. Restore excludes before materializations. Do not create or repair materializations for detached items; maintain only their exclude protection. If either a repo entry or the current managed block cannot be inspected safely while building the desired set, return an error without rewriting the block.

### `item sync`

Synchronize a regular copy only in an explicit direction.

```sh
shelfbox item sync <PATH> --from store [--dry-run]
shelfbox item sync <PATH> --from repo [--dry-run] --yes
```

Direction is required, and specifying both directions is rejected. Decisions are based on facts and observed materialization, not configured strategy.

#### `sync --from store`

* Treat the store as authoritative and atomically replace the existing repo copy.
* Operate only on `RegularCopy`.
* Equal content is a no-op. Diverged content may be overwritten because direction was explicit.
* Make the repo copy's content and permissions match the store.
* Reject `Missing` and suggest `item repair`.
* `ManagedSymlink` is a no-op.
* Reject hardlinks, unexpected/unsupported entries, tracked state, missing excludes, missing store, and inspection failures.

#### `sync --from repo`

* Treat the repo copy as authoritative and atomically replace the store.
* Operate only on an `attached` manifest item's `RegularCopy`.
* Equal content is a no-op.
* Preserve existing store permissions and replace only content.
* Reject managed symlinks, missing entries, hardlinks, unexpected/unsupported entries, tracked state, missing excludes, missing store, and inspection failures.
* Revalidate repo/store containment and file identity immediately before commit.
* Require `--yes` for an actual write. `--dry-run` returns the plan and facts without requiring `--yes`.

### `item restore`

Restore removes managed state; it is not a path for propagating edits.

* Managed symlink: restore a regular file from the store as in existing behavior.
* Equal regular copy: retain the repo copy and remove store/manifest management.
* Diverged regular copy: reject by default and require `item sync --from repo` or `item sync --from store` first.
* Missing, unsafe, or unexpected entries and a missing store are rejected.
* Normal restore removes management from the manifest/store and removes the exclude unless `--keep-ignore` is specified.
* Do not suggest `restore + add` as a way to propagate edits.

Protect store-data deletion and manifest updates with durable recovery. Where
possible, rename the store file into a recovery temp on the same filesystem
before updating the manifest, then delete the temp after commit.

#### `restore --keep-store`

`restore --keep-store` remains a legacy detach operation for compatibility. Unlike normal restore, it does not return the repo side to a regular file.

* Retain the manifest entry, store, and repo materialization, and perform only `attached -> detached`.
* Preserve the observed strategy: a symlink remains a symlink and a copy remains a copy.
* Always preserve the exclude, regardless of symlink/copy strategy.
* Treat `--keep-ignore` as implicitly enabled when `--keep-store` is specified.
* Do not change content.

`--keep-ignore` remains valid for normal restore. CLI help and the reference
must describe the detach semantics clearly so the command name is not mistaken
for normal restore.

### `item move`

* Move only a managed symlink or equal regular copy.
* Reject a diverged regular copy and require an `item sync` with explicit direction first.
* Preserve the healthy observed strategy at the destination.
* If the source materialization is missing, reject and require `item repair` first.
* Add and verify the destination exclude before writing, and reject tracked, occupied, or unsafe paths.
* Apply store, repo, manifest, and exclude updates as one recoverable operation.
* Preserve the same safety guarantees for cross-device store moves.

### `item relink`

Keep relink separate from normal sync because it performs a `detached -> attached` ownership transition.

* Add and verify the exclude entry before attach. Do not require the user to run `repo repair` because the exclude is missing.
* Missing repo path: materialize with the configured strategy, then attach.
* Valid symlink or equal regular copy: preserve the observed strategy and attach.
* Reject unsafe, unexpected, or unsupported entries.
* Reject a diverged regular copy by default.

Detached items are not eligible for `sync --from repo`, so only relink itself provides an explicit direction when a diverged copy must be resolved.

```sh
shelfbox item relink <PATH> --from store [--dry-run]
shelfbox item relink <PATH> --from repo [--dry-run] --yes
```

`--from store` atomically replaces the repo copy before attach. `--from repo` atomically replaces the store before attach. Require `--yes` for actual `--from repo` writes, but not for `--dry-run`.

Both directions reject tracked state, a missing store, and path/file-identity
violations. Add and verify the exclude first, then complete the direction as a
recoverable operation. When equal content is relinked without a direction,
attach as before.

## Relationship to Existing Features

### `repo reclaim`

Preserve existing ownership decisions regardless of copy/symlink strategy. Reclaim itself does not convert materializations. After reclaim, `repo repair` restores a missing materialization with the configured strategy.

### `store gc` / `rebuild-index`

No change. Do not use repo-copy presence or divergence in deletion decisions. When an unfinished operation exists, GC does not make related paths deletion candidates and requires recovery.

### `store verify`

Do not reduce its existing scope.

* Verify the presence and safety of every manifest and canonical store file.
* When an index entry has an associated local repo, also verify repo-side materialization through facts/policy.
* Support both symlinks and copies, reporting copy divergence and exclude/Git state.
* Continue checking the canonical store when a local repo entry is unavailable.
* Apply the same classifications as `item status` and `repo status`.
* Preserve separate `WARNING` and `ERROR` labels in CLI output. Either label
  returns exit code `2` for `store verify`.

## Explicit Conversion and Repository Operations

### `item materialize`

Convert one healthy materialization explicitly.

```sh
shelfbox item materialize <PATH> --strategy copy [--dry-run]
shelfbox item materialize <PATH> --strategy symlink [--dry-run]
```

* Symlink -> copy: create a durable, excluded temp copy from the store.
* Equal copy -> symlink: create and validate a replacement symlink.
* Diverged copy -> symlink: reject and require `item sync` with explicit direction first.
* If target strategy matches observed materialization, no-op.
* Do not change manifest identity or `ownership_state`.
* Reject tracked state, missing exclude, missing store, and unsafe/unexpected entries.

Do not delete the existing materialization first.

```text
create temp copy / temp symlink from store in the same directory
-> verify new materialization and file identity
-> atomically replace existing entry
```

If replacement cannot preserve the old entry, fail without converting it.

### Repository-wide operations

* `repo sync --from store|repo`
* `repo materialize --strategy symlink|copy`
* Apply item-level validation, conflict behavior, and reports.
* Validate the complete attached target set before the first write and require
  `--yes` when the selected sync direction would overwrite any regular-copy
  target. `--dry-run` reports the full target list without writing.
* Execute validated items in lexical order. Stop at the first execution-time
  failure and report completed earlier items; never write later items.
* Detached items remain outside repository batches and require an explicit
  item-level operation.

## Positioning

```text
symlink mode:
  store file is directly visible through repo path.

copy mode:
  store file is canonical.
  repo file is an editable materialized copy.
  repo edits become canonical only after an explicit --from repo operation.
```

This boundary keeps copy mode an optional materialization for restricted
environments without changing the meaning of ownership, repair, reclaim, or
GC.
