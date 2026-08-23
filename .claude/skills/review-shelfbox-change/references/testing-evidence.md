# Testing Evidence for Shelfbox Reviews

Choose the narrowest test layer that can prove the contract, then require a higher-level test when the user-visible behavior or public API changes. Do not accept a private unit test as the sole evidence for an observable CLI or compatibility contract.

| Test location | Suitable evidence |
| --- | --- |
| A module's `#[cfg(test)]` tests | Private algorithms, branch behavior, and local invariants. |
| `crates/shelfbox-core/src/integration_tests/` | Operation and recovery scenarios that need crate-private access, failure injection, or internal adapters. |
| `crates/shelfbox-core/tests/` | Public API behavior, external-consumer compatibility, architecture boundaries, and public JSON fixtures. |
| `crates/shelfbox/tests/` | CLI commands, user-visible output and exit status, and end-to-end filesystem behavior. |

## Review rules

- A changed CLI command needs CLI-level evidence unless the change is provably unreachable to users.
- A changed public core API or serialized contract needs an external-core test or a compatibility fixture when it represents a supported contract.
- A private implementation refactor may use focused internal tests when it leaves all observable contracts unchanged.
- Place tests needing crate-private access inside `shelfbox-core`; do not broaden the public API only to make a test compile.
- Keep fixtures for intentional stable contracts, not incidental formatting or implementation details.

For filesystem mutations, seek the applicable successful-operation, zero-mutation dry-run, protected-existing-target, containment-failure, and failure-aftermath cases. A test that checks only an error code is insufficient when the contract constrains retained filesystem or metadata state.
