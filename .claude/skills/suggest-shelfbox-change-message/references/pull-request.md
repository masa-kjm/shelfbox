# Pull-Request Messages

Use the recommended squash-merge commit subject as the pull-request title unless the pull request has a clearly different, broader user-facing outcome.
The title must match the diff and remain concise, specific, and in English.

Propose a pull-request body in this form, omitting sections that do not apply:

```markdown
## Summary

- <behavioral outcome>

## Safety and compatibility

- <affected invariant or compatibility contract>

## Acceptance criteria

| Criterion | Evidence |
| --- | --- |
| <criterion> | <test, code path, or other reviewable evidence> |

## Tests

- `<command>` — <result>
- Not run: <command> — <reason>

Closes #N
```

Describe only behavior, safety properties, compatibility effects, and validation supported by the diff or supplied evidence.
Do not claim that a command passed unless its result is available.
If the change does not affect a safety or compatibility contract, say so only when that conclusion is supported by the changed behavior.

Include the acceptance-criteria mapping for behavior changes that affect filesystem mutations, ownership, persistence, recovery, or compatibility.
For lower-risk changes, a concise summary and test evidence are sufficient.

Use `Closes #N` only in the pull-request body and only for an issue actually addressed by the pull request.
If no issue number is known, omit the line rather than inventing one.
