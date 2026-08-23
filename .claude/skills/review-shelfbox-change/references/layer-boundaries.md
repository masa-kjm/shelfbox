# Layer-boundary Review Evidence

The authoritative boundary definition is [`docs/architecture/module-map.md`](../../../../docs/architecture/module-map.md). Use this reference for changes that cross the CLI-to-core API boundary; it is not a second architecture specification.

## Review scope

Examine the boundary among:

- `crates/shelfbox/src/cli.rs`
- `crates/shelfbox/src/commands/`
- `crates/shelfbox-core/src/api/`

Check that parsing, dispatch, prompts, terminal formatting, and exit-code mapping remain in the CLI crate. Production commands must invoke `shelfbox_core::api` for behavior rather than reproducing validation, filesystem, storage, ownership, recovery, dry-run, or policy decisions.

Public core operations should expose deliberate API entry points and return plans or reports for the CLI to format. Do not require new public exports merely to test implementation details; tests that need crate-private access belong inside `shelfbox-core`.

## Evidence to seek

- The complete path from changed CLI input to the owning core operation.
- No duplicate safety or ownership decision in command code.
- API result data sufficient for the CLI to format the intended user contract without core printing terminal output.
- Tests at the owner of the changed behavior, plus CLI-level evidence when observable command behavior changes.

Add a new review rule here only after it becomes a stable, reusable concern. Link to the architecture source of truth instead of copying its module inventory or dependency graph.
