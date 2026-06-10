# Elara Codex Workflow

Use this file by reference from `docs/CODEX_GOAL.md`; do not paste it into the
`/goal` prompt.

## Work Cycle

At the start of each work cycle, read only the relevant parts of:

- `docs/PROGRESS.md` for the current milestone, next incomplete step, known
  gaps, and last verification.
- `docs/MILESTONES.md` for the current milestone step plan.
- `docs/ARCHITECTURE.md` for touched boundaries and invariants.
- `~/Downloads/lua-lua-a5522f0` for official Lua 5.5 behavior when changing Lua
  semantics.

Implement one small, verifiable unit at a time. Add focused tests, run
formatting and the narrowest meaningful verification, update
`docs/PROGRESS.md` as rolling status, and commit each completed unit with a
conventional commit message.

Continue across commits and milestones until all milestones are complete, the
user asks to stop, or a genuine blocker appears. Do not mark the `/goal`
complete earlier.

## Project Rules

- Keep the architecture clean and layered.
- Do not add compatibility flags for old Lua versions unless
  `docs/ARCHITECTURE.md` is explicitly revised.
- Commit by verifiable step, not by whole milestone.
- Avoid large working-tree changes and unrelated refactors.
- Keep tests near the code they verify.
- Keep source files under 1000 lines; prefer modules below 600 lines when
  practical.
- Unsafe Rust must be localized and documented with `SAFETY` comments.
- Public APIs must not expose unrooted raw GC pointers.
- JIT must remain optional and semantically equivalent to the interpreter for
  supported paths.
