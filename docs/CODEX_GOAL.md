# Elara Codex Goal

Status: Draft 2
Purpose: compact prompt templates for long-running Codex work on Elara.

This document is intentionally short. Detailed project rules live in the
canonical project documents:

- `docs/ARCHITECTURE.md`: language target, Lua 5.5 references, layering,
  public API safety, file-size guidance, unsafe policy, JIT invariants.
- `docs/MILESTONES.md`: milestone order, expected changes, verification
  commands, and commit messages.
- `docs/PROGRESS.md`: current milestone, current step, completed work,
  remaining gaps, and last verification.

## Primary `/goal` Prompt

Use this prompt when Codex should continue the whole project automatically.

```text
/goal
You are working on Elara, a Rust-native implementation of the current stable Lua VM with a high-level Rust embedding API and optional Cranelift JIT.

Continue implementing `docs/MILESTONES.md` step by step until every milestone is complete.

At the start of each work cycle, read `docs/PROGRESS.md`, the next relevant section of `docs/MILESTONES.md`, the relevant boundaries in `docs/ARCHITECTURE.md`, and this `docs/CODEX_GOAL.md`.

When a step changes Lua semantics, inspect the relevant official Lua 5.5 source under `~/Downloads/lua-lua-a5522f0` and the Lua 5.5 manual before designing the change. Use them as behavior references, while keeping Elara's Rust-native layered architecture and custom bytecode.

Work in clean, verifiable units: implement one milestone step, or a smaller sub-step when needed; add or update tests; run the narrowest meaningful verification; update `docs/PROGRESS.md`; commit with the conventional commit message specified or implied by `docs/MILESTONES.md`; then continue to the next incomplete step.

Do not stop after one step or one milestone. Stop only when all milestones are complete, the user asks you to stop, or you are genuinely blocked by a missing architecture decision, an unfixable verification failure, or unavailable required context.

Do not mark this `/goal` complete until every milestone in `docs/MILESTONES.md` is complete. At the end of each assistant turn, report the completed step, commit, verification, and next step.
```

## Focused Step Prompt

Use this prompt when Codex should complete only the next step.

```text
/goal
Continue Elara by implementing exactly one verifiable step from `docs/MILESTONES.md`.

Read `docs/PROGRESS.md`, the relevant milestone text in `docs/MILESTONES.md`, the relevant boundaries in `docs/ARCHITECTURE.md`, and this `docs/CODEX_GOAL.md`. If Lua behavior is involved, inspect the relevant official Lua 5.5 source under `~/Downloads/lua-lua-a5522f0`.

Choose the next incomplete step from `docs/PROGRESS.md`. Keep the change small, add tests, run the narrowest meaningful verification, update `docs/PROGRESS.md`, and commit with a conventional commit message. Do not implement unrelated future steps.
```

## Cleanup Prompt

Use this prompt when the repository needs stabilization before feature work.

```text
/goal
Stabilize the Elara repository without adding new features.

Read `docs/PROGRESS.md`, `docs/MILESTONES.md`, `docs/ARCHITECTURE.md`, and this `docs/CODEX_GOAL.md`. If behavior correctness is involved, inspect the relevant official Lua 5.5 source under `~/Downloads/lua-lua-a5522f0`.

Focus only on restoring a clean, reviewable state: run formatting and the smallest useful failing test, fix compilation or test failures, update `docs/PROGRESS.md` if status changed, and commit with a conventional cleanup message.
```

## Operating Contract

When using the primary prompt:

- `docs/PROGRESS.md` decides the next step.
- `docs/MILESTONES.md` defines scope, verification, and commit shape.
- `docs/ARCHITECTURE.md` defines non-negotiable boundaries and invariants.
- The local Lua 5.5 source tree and manual define Lua behavior.
- Each completed step must be testable, documented in `docs/PROGRESS.md`, and
  committed before continuing.
- `docs/PROGRESS.md` remains rolling status, not a changelog.
- Unrelated refactors and large mixed commits are out of scope.
- Existing user changes in the worktree must not be reverted unless explicitly
  requested.

If these documents conflict, pause and make the conflict explicit before
changing architecture or widening the implementation scope.
