# Elara Codex Goal

Status: Draft 6
Purpose: keep the reusable `/goal` prompt minimal. Detailed project rules live
in the referenced docs and must be read on demand.

## Prompt To Paste

```text
/goal Continue Elara from docs/PROGRESS.md. Follow docs/CODEX_GOAL.md by
reference until every milestone in docs/MILESTONES.md is complete.
```

## Operating Contract

Do not expand the `/goal` prompt with the full project plan. For each work
cycle, read only the relevant parts of these references:

- `docs/PROGRESS.md`: current milestone, next incomplete step, known gaps, last
  verification.
- `docs/MILESTONES.md`: current milestone step plan.
- `docs/ARCHITECTURE.md`: boundaries and invariants touched by the change.
- `~/Downloads/lua-lua-a5522f0`: official Lua 5.5 source for behavior.

Implement one small, verifiable unit at a time. Add focused tests, run the
narrowest meaningful verification plus formatting, update `docs/PROGRESS.md` as
rolling status, and commit each completed unit with a conventional commit
message.

Continue across commits and milestones until all milestones are complete, the
user asks to stop, or a genuine blocker appears. Do not mark the `/goal`
complete earlier.
