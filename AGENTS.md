# shelfbox Agent Instructions

Follow these instructions for every change in this repository.
Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before work that affects issues, branches, commits, pull requests, reviews, or releases; it is the source of truth for that workflow.

## Sources of Truth

- Before changing behavior, read the relevant documents linked from [`docs/index.md`](docs/index.md), including the applicable specification and architecture documents.
- Treat an issue's acceptance criteria and the relevant `docs/spec/` contract as the behavioral source of truth. Do not infer an intentional behavior change from an implementation detail or a comment.
- If requirements conflict or leave a safety-, ownership-, persistence-, or compatibility-sensitive choice unclear, report the ambiguity before making that choice.

## Engineering

- Write code, code comments, user-facing documentation, commit messages, and pull-request text in English.
- In comments and doc comments you write or touch, do not manually wrap a single sentence across multiple lines; keep one sentence per line instead. Do not reflow untouched existing comments just to satisfy this.
- Make appropriate changes while maintaining scalability, maintainability, compatibility, and safety.
- Keep the Rust architecture boundaries in [`docs/architecture/module-map.md`](docs/architecture/module-map.md): CLI commands format and dispatch through `shelfbox_core::api`; core operations own behavior; policies remain free of I/O.
- Preserve Shelfbox's safety guarantees. In particular, do not follow or replace unexpected filesystem entries, write outside a trusted root, or make dry-run paths mutate state. Treat ownership, store contents, manifests, and unrelated materializations as protected during failures.
- Add or update behavior-level tests for changed user-visible behavior and safety invariants. Update documentation and compatibility fixtures when a user-facing contract changes.
- Do not make external or irreversible changes, including publishing, merging, pushing, deleting user data, or changing GitHub state, unless the task explicitly authorizes them.

## Reviews

- Review behavior changes against the linked issue's acceptance criteria, the relevant specification, architecture boundaries, safety guarantees, and compatibility contracts.
- Map each acceptance criterion to implementation and test evidence, and mark it as verified, unverified, or violated.
- Report only evidence-backed findings with a concrete affected location. Do not edit the reviewed implementation unless the task explicitly asks for changes.

## Validation

Run the relevant checks before handoff when the environment permits:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Report commands that could not run and why. Never claim an unrun check passed.

## Collaboration

Keep the user informed of material plan changes, specification ambiguities, validation limitations, and actions that need additional authority. Make safe, reversible progress when the request is clear; ask before making a decision that changes the requested behavior or affects external state.
