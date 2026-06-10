# Elara Codex Goal

Status: Draft 5
Purpose: keep the reusable `/goal` prompt small enough for long-running Codex
work while preserving the project rules by reference.

## Prompt To Paste

```text
/goal Continue Elara by following docs/CODEX_GOAL.md until every milestone in
docs/MILESTONES.md is complete.
```

## Reference Reading

Do not paste the full project rules into the `/goal` prompt. Use these files as
references and read only what is needed for the current step:

- `docs/PROGRESS.md`: current milestone, next incomplete step, known gaps, last
  verification.
- `docs/MILESTONES.md`: only the section for the current milestone and step.
- `docs/ARCHITECTURE.md`: only the boundaries and invariants touched by the
  current change.
- `~/Downloads/lua-lua-a5522f0`: local official Lua source for actual Lua 5.5
  behavior and implementation details.

## Step Loop

For each unit of work:

1. Read `docs/PROGRESS.md` to find the current milestone and next incomplete
   step.
2. Read only the relevant `docs/MILESTONES.md` step and touched
   `docs/ARCHITECTURE.md` boundaries.
3. Check the matching official Lua source under `~/Downloads/lua-lua-a5522f0`
   when implementing Lua behavior.
4. Implement one verifiable step, or a smaller reviewable sub-step.
5. Add or update focused tests.
6. Run formatting and the narrowest meaningful verification.
7. Update `docs/PROGRESS.md` as rolling status, not a changelog.
8. Commit with a conventional commit message.
9. Continue until all milestones are complete, the user asks to stop, or a
   genuine blocker appears.

## Rules

- Preserve the documented architecture and layering.
- Do not add old-Lua compatibility flags unless `docs/ARCHITECTURE.md` is
  revised first.
- Do not mark the `/goal` complete until every milestone is done.
- If referenced docs conflict, stop and report the conflict explicitly.
