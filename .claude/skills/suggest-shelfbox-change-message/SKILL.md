---
name: suggest-shelfbox-change-message
description: Propose evidence-based Conventional Commit messages and pull-request titles or bodies for a Shelfbox change. Use when naming a branch or pull request, choosing a squash-merge commit message, or checking that the proposed wording matches the diff.
---

# Suggest Shelfbox Change Messages

Propose messages only; do not create commits, alter Git history, or open or edit a pull request.
Follow `CONTRIBUTING.md` as the source of truth.
Git is required; GitHub issue lookup is optional and requires an authenticated `gh` CLI.

## Gather evidence

1. Read the current branch name, any supplied commit message or pull-request title, and the diff against the proposed base (normally `main`). Inspect changed paths and the relevant diff; do not infer intent from a branch name alone.
2. Extract every literal `#N` reference from the branch name and supplied titles. If at least one exists, use the helper:

   ```sh
   scripts/linked-issues.sh \
     --branch "<branch>" --title "<proposed title>"
   ```

   The helper runs `gh issue view` only when a reference is present. Read the issue title and acceptance criteria to validate the proposed wording. If `gh` is unavailable, unauthenticated, or the issue cannot be read, state that limitation and continue from the local diff. Do not invent an issue number.
3. Read the reference that matches the requested output:

   - For a commit subject or body, read [commit-message.md](references/commit-message.md).
   - For a pull-request title or body, read [pull-request.md](references/pull-request.md).
   - When the requested output is unspecified, propose both a squash-merge commit and a pull-request title and body, and read both references.

## Respond

State the proposed message or messages, then list the branch, changed behavior, and inspected issue criteria as evidence.
Only offer an alternative when the type, scope, or pull-request framing is genuinely ambiguous, and explain the distinction in one sentence.
If the diff contains unrelated changes, identify that as a PR-scoping concern instead of hiding it behind a broad message.
