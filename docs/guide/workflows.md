# Common Workflows

This document describes common shelfbox tasks and recovery procedures.

For command details, see the documents under [`../reference/`](../reference/).

---

## Workflow Index

* Get started
  * [Start Managing a Local File](#start-managing-a-local-file)
* Manage local-only files
  * [Make an Already-Tracked File Local Only](#make-an-already-tracked-file-local-only)
  * [Restore a Shelved File](#restore-a-shelved-file)
  * [Restore All Shelved Items](#restore-all-shelved-items)
  * [Shelve or Restore a Directory](#shelve-or-restore-a-directory)
  * [Rename a Shelved Path](#rename-a-shelved-path)
  * [Reattach a Detached Item](#reattach-a-detached-item)
* Repair and synchronize working trees
  * [Repair Materializations and Git Excludes](#repair-materializations-and-git-excludes)
  * [Use Copy Mode and Resolve an Edit](#use-copy-mode-and-resolve-an-edit)
  * [Convert a Materialization Strategy](#convert-a-materialization-strategy)
  * [Synchronize a Repository Batch](#synchronize-a-repository-batch)
* Work with repository copies
  * [Recover After Repository Move](#recover-after-repository-move)
  * [Use a Linked Git Worktree](#use-a-linked-git-worktree)
  * [Keep Multiple Clones Separate](#keep-multiple-clones-separate)
  * [Recover After Reclone](#recover-after-reclone)
* Recover and maintain the store
  * [Recover After Local Index Loss](#recover-after-local-index-loss)
  * [Move the Store](#move-the-store)
  * [Store/Metadata Recovery from repos/](#storemetadata-recovery-from-repos)
  * [Audit the Store and Clean Orphaned Data](#audit-the-store-and-clean-orphaned-data)
  * [Troubleshooting](#troubleshooting)
  * [Advanced Diagnostics](#advanced-diagnostics)

---

## Start Managing a Local File

Use this workflow to keep an untracked local file available in your repository
without allowing Git to track it. For a file that Git already tracks, follow
[Make an Already-Tracked File Local Only](#make-an-already-tracked-file-local-only)
instead.

Create a local file from the repository root:

```sh
echo "my local note" > local.md
git status --short
```

Confirm that `local.md` is untracked, then shelve it:

```sh
shelfbox item add local.md
```

The file remains available at its original path:

```sh
cat local.md
```

But Git no longer reports it:

```sh
git status --short
```

shelfbox moves the canonical content to its store, materializes it at the
original path as a symlink by default, and adds the path to
.git/info/exclude. Your editor and other tools can continue to use the file
at the same path.

Verify that shelfbox is managing the item and repository:

```sh
shelfbox item list
shelfbox repo list
```

To return the file to ordinary repository management, run:

```sh
shelfbox item restore local.md
```

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)

---

## Make an Already-Tracked File Local Only

`item add` accepts only files that Git does not track. Use this migration only
after the repository's maintainers have agreed that the file should no longer
be part of the project. Keep a committed template such as `.env.example` when
other developers or automation still need an example.

```sh
git rm --cached .env
shelfbox item add .env
git commit -m "Stop tracking local environment configuration"
```

`git rm --cached` stages removal from the repository but leaves the working
tree file in place. Review the staged deletion before committing: future
clones will no longer receive this file from Git.

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)

---

## Restore a Shelved File

Example:

```sh
shelfbox item restore local.md
```

Use this when the file should become a normal repository file again.

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)

---

## Restore All Shelved Items

To return every item managed in the current repository to ordinary files, first
review the selected paths without writing:

```sh
shelfbox item restore --all --dry-run
```

Then restore the complete managed set:

```sh
shelfbox item restore --all
```

This command is limited to the current repository's manifest; it does not
restore items from other repositories in the same store. Each item follows the
ordinary restore safety checks. If an item fails, earlier successful restores
remain in effect and shelfbox reports the path that needs attention.

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)

---

## Shelve or Restore a Directory

Shelve the eligible files below a directory while preserving their relative
paths. Each file is an independent item; the directory is only a convenient
way to select them.

```sh
shelfbox item add secrets/ --dry-run
shelfbox item add secrets/
```

Nested Git repositories are not crossed. shelfbox reports files that it skips
or cannot shelve. A later failure does not roll back files that were already
shelved, so check the per-file summary before continuing.

Restore all managed items below the same directory with a trailing slash:

```sh
shelfbox item restore secrets/ --dry-run
shelfbox item restore secrets/
```

Directory restore applies the ordinary file restore policy to every selected
item. It likewise reports each result and keeps any items restored before a
later failure; resolve the reported path and run the command again if needed.

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)

---

## Rename a Shelved Path

Move a managed file when the repository path changes. This preserves its
ownership and materialization strategy without restoring and shelving it
again.

```sh
shelfbox item move config/local.yml config/override.yml --dry-run
shelfbox item move config/local.yml config/override.yml
```

The command moves the canonical store file and materialization, then updates
the manifest and the shelfbox block in `.git/info/exclude`.

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)

---

## Repair Materializations and Git Excludes

Symptoms:

```text
item status
repo status
```

reports a missing materialization, invalid symlink, or missing Git exclude
entry.

Repair a missing item materialization:

```sh
shelfbox item repair <PATH>
```

Repair all missing materializations and rebuild the shelfbox-managed exclude
block for the current repository:

```sh
shelfbox repo repair
```

For a symlink that points to an unexpected target, inspect the target first
and replace it only when that is intentional:

```sh
shelfbox item repair --force <PATH>
```

If an unexpected regular file occupies the path, shelfbox does not overwrite
it. Decide whether to preserve, move, or remove that file manually before
repairing. In Copy mode, synchronize a diverged copy with `item sync` instead
of repairing it.

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)
* [`../spec/failure-matrix.md`](../spec/failure-matrix.md)

---

## Recover After Local Index Loss

Symptoms:

```text
index.json missing or empty
repos/*/manifest.json still exists
```

Recovery:

```sh
shelfbox store rebuild-index
shelfbox repo reclaim
shelfbox repo repair
```

`repos/` is the canonical store; `index.json` is only a local cache.

If `manifest.json` itself is missing or corrupted, restore it from backup or
repair it manually before running `store rebuild-index`. shelfbox cannot infer
canonical ownership safely from loose files under `items/` alone.

See:

* [`../reference/repo-commands.md`](../reference/repo-commands.md)
* [`../spec/failure-matrix.md`](../spec/failure-matrix.md)

---

## Recover After Repository Move

Symptoms:

```text
Repository path changed.
```

Recovery:

```sh
shelfbox repo repair
```

This refreshes local repository metadata and repairs symlinks/exclude entries
when the current clone is already associated with the existing `RepoId`.

See:

* [`../reference/repo-commands.md`](../reference/repo-commands.md)
* [`../spec/ownership-model.md`](../spec/ownership-model.md)

---

## Use a Linked Git Worktree

Linked worktrees created with `git worktree add` share the same Git common
directory, so shelfbox treats them as one repository identity and one shelf.
This differs from independently cloned directories, which remain separate by
default.

```sh
git worktree add ../project-feature feature
cd ../project-feature
shelfbox repo repair
```

Run `repo repair` in a newly created worktree to materialize the existing
attached items there and repair that worktree's Git exclude integration. Do
not use `repo reclaim` for a linked worktree.

With Copy mode, each worktree has a separate regular-file materialization.
Use `item status` and `item sync` to resolve any copy that diverges from the
canonical store content.

See:

* [`../reference/repo-commands.md`](../reference/repo-commands.md)
* [`../spec/ownership-model.md`](../spec/ownership-model.md)

---

## Keep Multiple Clones Separate

When the same Git repository is cloned into multiple directories, shelfbox
treats each independent clone as a separate repository by default. Each clone
gets its own `RepoId` and its own canonical shelf data when you add an item in
that clone.

For example, to keep different local configuration in two clones:

```sh
cd ~/src/project-main
shelfbox item add local.md

cd ~/src/project-experiment
shelfbox item add local.md
```

Do not run `shelfbox repo reclaim` in the second clone. Reclaim transfers the
existing shelf association to the current clone; it is a recovery operation,
not a way to share one shelf between concurrently used independent clones.

If you need the original shelved files in a replacement clone, follow the
reclone recovery procedure below instead.

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)
* [`../reference/repo-commands.md`](../reference/repo-commands.md)
* [`../spec/ownership-model.md`](../spec/ownership-model.md)

---

## Recover After Reclone

Symptoms:

```text
Repository was cloned again.
Old shelved items still exist.
```

Run these commands from the new clone. If the store and its `index.json` are
still present, re-associate the replacement clone and restore its working-tree
integration:

```sh
shelfbox repo reclaim
shelfbox repo repair
```

Do not run `item add` before reclaiming: that creates a separate `RepoId`, and
reclaim refuses a clone that already has managed items.

If `index.json` was lost, is empty, or the store was restored on another
machine, rebuild the local cache first:

```sh
shelfbox store rebuild-index
shelfbox repo reclaim
shelfbox repo repair
```

`item add` and `repo status` may print a reclaim hint when the new clone has no
local cache match but existing manifests match by hints. The hint is only a
guide; run `repo reclaim` to attach the clone explicitly.

Reclaim does not copy or merge item data. It changes the existing shelf's
current-clone association, so use it when the new clone replaces the previous
one rather than to share a shelf between independently used clones.

See:

* [`../reference/repo-commands.md`](../reference/repo-commands.md)
* [`../spec/ownership-model.md`](../spec/ownership-model.md)
* [`../spec/failure-matrix.md`](../spec/failure-matrix.md)

---

## Move the Store

1. Move the store directory.
2. Update configuration.
3. Run repository repair if needed.

Example:

```sh
mv ~/.local/share/shelfbox /new/location/shelfbox
```

Then update configuration:

```sh
shelfbox config set store /new/location/shelfbox
```

Changing the active store does not automatically move repository ownership or
materializations to the new location. A repository previously managed under a
different store must be re-associated by restoring the original `repos/` data,
rebuilding the local index if needed, and then running `repo reclaim` and
`repo repair`.

See:

* [`../reference/config-commands.md`](../reference/config-commands.md)
* [`../reference/repo-commands.md`](../reference/repo-commands.md)

---

## Reattach a Detached Item

A detached item is created by:

```sh
shelfbox item restore --keep-store
```

Reattach it:

```sh
shelfbox item relink <PATH>
```

See:

* [`../reference/item-commands.md`](../reference/item-commands.md)
* [`../spec/ownership-model.md`](../spec/ownership-model.md)

---

## Use Copy Mode and Resolve an Edit

Enable Copy mode before creating a new item when symlinks cannot be created:

```sh
shelfbox config set materialization copy
shelfbox item add local.md
```

The repository path is then an independent regular file. It remains protected
by `.git/info/exclude`, but an edit is not automatically canonical. Inspect and
choose exactly one direction:

```sh
shelfbox item status
shelfbox item sync local.md --from store       # replace the repo copy
shelfbox item sync local.md --from repo --yes  # replace canonical store content
```

`item repair`, `item move`, and normal `item restore` refuse to silently
overwrite a diverged copy. Synchronize first. A missing exclude entry for a
Copy item is an error; run `shelfbox repo repair` before a content mutation.

---

## Convert a Materialization Strategy

Use an explicit conversion when an existing item should change between a
symlink and a regular Copy. Changing `materialization` configuration alone
does not convert existing items.

```sh
shelfbox item materialize local.md --strategy copy
shelfbox item materialize local.md --strategy symlink
```

The old entry remains in place until a validated replacement is ready. A
diverged regular copy must be synchronized explicitly before converting it to
a symlink.

For all attached items, preview and then execute the corresponding batch:

```sh
shelfbox repo materialize --strategy copy --dry-run
shelfbox repo materialize --strategy copy
```

---

## Synchronize a Repository Batch

Review all attached items without writing, then choose a source of truth for
the batch:

```sh
shelfbox repo sync --from store --dry-run
shelfbox repo sync --from store --yes
shelfbox repo sync --from repo --yes
```

Both directions require `--yes` when any item would be overwritten by the
selected source of truth. Without `--yes`, shelfbox returns a
confirmation-required error and does not prompt interactively. When no item
needs replacement, either direction runs without `--yes`. The batch validates
every item before its first write. If an execution-time race occurs later, it
stops and reports the completed items rather than continuing into unvalidated
state.

---

## Store/Metadata Recovery from repos/

Use this workflow after a PC migration, a reclone, or a local index loss when
`repos/` under the active store path is still present. It restores the
canonical repository metadata first, then re-associates the current clone and
repairs local working-tree state:

```sh
shelfbox store rebuild-index
shelfbox repo reclaim
shelfbox repo repair
```

Run `shelfbox repo reclaim` and `shelfbox repo repair` from the working tree of
each repository you want to re-associate. Repeat those two commands per
repository after rebuilding the store index.

This associates the current clone with the selected existing `RepoId` and then
repairs local symlinks and Git exclude entries. Keep the original `repos/`
contents intact until the re-association succeeds; a store move or restore is a
recovery workflow, not an automatic multi-store migration. If a repository was
previously managed in another store, do not treat the new store as a shared
live shelf for the same repo.

See:

* [`../reference/repo-commands.md`](../reference/repo-commands.md)
* [`../reference/store-commands.md`](../reference/store-commands.md)
* [`../spec/ownership-model.md`](../spec/ownership-model.md)

---

## Audit the Store and Clean Orphaned Data

Check canonical store data and the currently associated local checkout:

```sh
shelfbox store verify
```

Preview conservative cleanup before deleting anything:

```sh
shelfbox store gc --dry-run
shelfbox store gc --yes
```

Only items explicitly classified as `orphaned` are eligible for deletion.
`attached`, `detached`, and `unreachable` items remain protected; a missing
clone or a rebuilt index is never a deletion signal.

`store gc` cannot recover a missing store file or a corrupted manifest.
Restore those from backup (or repair a manifest manually) before running
`store rebuild-index` or attempting further recovery.

See:

* [`../reference/store-commands.md`](../reference/store-commands.md)
* [`../spec/failure-matrix.md`](../spec/failure-matrix.md)

---

## Troubleshooting

Start with:

```sh
shelfbox repo status
```

Then consult:

* [`../spec/failure-matrix.md`](../spec/failure-matrix.md)
* [`../reference/item-commands.md`](../reference/item-commands.md)
* [`../reference/repo-commands.md`](../reference/repo-commands.md)

Most recovery procedures begin with repository status and repair operations.

---

## Advanced Diagnostics

Inspect runtime context and store state:

```sh
shelfbox internal debug
```

By default, paths under your home directory are masked as `~` for safer sharing.
Use `--allow-sensitive` only when raw absolute paths are required.

Generate shell completions:

```sh
# Bash
shelfbox internal completions bash >> ~/.bash_completion

# Zsh
shelfbox internal completions zsh > ~/.zsh/completions/_shelfbox

# Fish
shelfbox internal completions fish > ~/.config/fish/completions/shelfbox.fish
```
