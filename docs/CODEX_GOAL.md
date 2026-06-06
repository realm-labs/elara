# Elara Codex Goal

Status: Draft 3
Purpose: compact `/goal` prompts for long-running Codex work on Elara.

Detailed rules intentionally live outside the prompt:

- `docs/PROGRESS.md`: current milestone, next step, gaps, verification.
- `docs/MILESTONES.md`: step order, scope, tests, commit shape.
- `docs/ARCHITECTURE.md`: invariants, layering, Lua 5.5 references.

## Primary `/goal` Prompt

```text
/goal Continue Elara until every milestone is complete.
Read and follow `docs/CODEX_GOAL.md`, `docs/PROGRESS.md`,
`docs/MILESTONES.md`, and `docs/ARCHITECTURE.md`.
```

## Focused Step Prompt

```text
/goal Continue Elara by completing exactly the next verifiable step.
Read and follow `docs/CODEX_GOAL.md`, `docs/PROGRESS.md`,
`docs/MILESTONES.md`, and `docs/ARCHITECTURE.md`.
```

## Cleanup Prompt

```text
/goal Stabilize Elara without adding new features.
Read and follow `docs/CODEX_GOAL.md`, `docs/PROGRESS.md`,
`docs/MILESTONES.md`, and `docs/ARCHITECTURE.md`.
```

## Contract

- `docs/PROGRESS.md` decides the next incomplete step.
- For Lua semantics, inspect the Lua 5.5 manual and local official source at
  `~/Downloads/lua-lua-a5522f0` before designing behavior changes.
- Complete one verifiable step or smaller sub-step per commit.
- Add or update tests, run the narrowest meaningful verification, update
  `docs/PROGRESS.md`, and commit with a conventional message.
- Keep `docs/PROGRESS.md` as rolling status, not a changelog.
- Do not mix unrelated refactors with feature work or revert user changes.
- For the primary prompt, continue after each commit and stop only when all
  milestones are complete, the user asks to stop, or a genuine blocker appears.
- Do not mark the `/goal` complete until every milestone is done.
- If referenced docs conflict, pause and make the conflict explicit.
