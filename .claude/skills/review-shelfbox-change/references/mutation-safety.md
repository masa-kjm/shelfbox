# Mutation-safety Review Evidence

The authoritative safety and recovery contracts are the relevant documents in [`docs/spec/`](../../../../docs/spec/) and [`docs/architecture/`](../../../../docs/architecture/). Use this lens whenever a change can write, remove, replace, or recover filesystem or persistent state.

## Establish the operation contract

For each changed write path, identify its preconditions, trusted root, intended writes, commit point, dry-run result, and state that can remain after each failure. Verify that the change preserves canonical store content, ownership, manifests, indexes, and unrelated materializations unless the approved specification explicitly changes one of those contracts.

The baseline checks are:

- Unexpected regular files, directories, symlinks, junctions, and reparse points are neither followed nor replaced.
- A final symlink is replaceable only through the command's explicit force semantics, with fresh no-follow state before the commit.
- A lexical path check is not treated as containment proof; existing intermediate components must not redirect work outside the trusted root.
- Dry-run is fully side-effect free, including parent creation, materialization, metadata or ignore updates, journals, and cleanup.
- A newly introduced write has a considered failure aftermath. Cleanup must not delete user-visible artifacts unless the contract explicitly authorizes it.

When both symlink and Copy materializations are in scope, verify both. Symlink handling retains wrong-target and replacement protections; Copy handling preserves divergence and isolated-file protections, and repair never silently overwrites user-owned or diverged content.

For a batch operation, inspect planning, reports, metadata updates, and per-item execution. A failure must not make an unprocessed item appear repaired or corrupt a completed item.

## Evidence to seek

Prefer tests that observe filesystem state, not merely an error return. For each applicable risk, seek evidence for the successful operation, a zero-mutation dry run, protected existing targets, containment failure, and a failure after an introduced write. State why a risk is not applicable rather than silently omitting it.
