# Elara Codex Goal

Status: Draft 4
Purpose: keep the reusable `/goal` prompt small enough for long-running Codex
work while preserving the project rules by reference.

## Prompt To Paste

```text
/goal Continue Elara until every milestone in docs/MILESTONES.md is complete.
Follow docs/CODEX_GOAL.md. Use docs/PROGRESS.md to find the next incomplete
step, read only the relevant MILESTONES and ARCHITECTURE sections, verify,
update PROGRESS, commit one verifiable unit, then continue.
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

## Work Contract

- Implement the next verifiable step, or a smaller sub-step when the step is too
  large for one reviewable commit.
- Preserve the documented architecture and layering.
- Do not add old-Lua compatibility flags unless `docs/ARCHITECTURE.md` is
  revised first.
- Add or update focused tests for behavior changes.
- Run the narrowest meaningful verification before each commit.
- Update `docs/PROGRESS.md` as rolling status, not a changelog.
- Commit with a conventional commit message after each verified unit.
- Continue after each commit until all milestones are complete, the user asks to
  stop, or a genuine blocker appears.
- Do not mark the `/goal` complete until every milestone is done.
- If referenced docs conflict, stop and report the conflict explicitly.
