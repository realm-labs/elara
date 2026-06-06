# Elara Codex Goal

Status: Draft 1  
Purpose: prompt and operating rules for Codex or another coding agent working on Elara.

## Primary `/goal` Prompt

Use this compact prompt when starting or resuming Codex work in the Elara
repository. The prompt intentionally references this document for detailed rules
instead of copying every rule into the `/goal` body.

```text
/goal
You are working on Elara, a Rust-native implementation of the latest stable Lua VM with a high-level Rust embedding API and optional Cranelift JIT.

Continue implementing the project step by step until every milestone in `docs/MILESTONES.md` is complete.

At the start of each work cycle, read `docs/PROGRESS.md`, the relevant section of `docs/MILESTONES.md`, the relevant architecture boundaries in `docs/ARCHITECTURE.md`, and the operating rules in `docs/CODEX_GOAL.md`.

When a step changes Lua semantics, inspect the relevant official Lua 5.5 source files under `~/Downloads/lua-lua-a5522f0` before designing the change. Use the source and Lua 5.5 manual as behavior references, while preserving Elara's Rust-native layered architecture and custom bytecode.

Implement exactly one verifiable step at a time, or a smaller sub-step when needed. After each clean step, add/update tests, run the narrowest meaningful verification, update `docs/PROGRESS.md`, commit with a conventional commit message, and continue automatically to the next incomplete step.

Do not stop after one step or one milestone. Stop only when all milestones are complete, you are truly blocked, verification cannot be safely fixed, a required architecture decision is not covered by the docs, or the user explicitly asks you to stop.

Do not mark this `/goal` complete until every milestone in `docs/MILESTONES.md` is complete. At the end of each assistant turn, report the completed step, commit, verification, and next step.
```

## Secondary `/goal` Prompt for Focused Steps

Use this when Codex should work on one specific step.

```text
/goal
Continue Elara by implementing exactly one verifiable step from docs/MILESTONES.md.

Read `docs/ARCHITECTURE.md`, `docs/MILESTONES.md`, `docs/PROGRESS.md`, and the operating rules in `docs/CODEX_GOAL.md`. When the step changes Lua semantics, inspect the relevant official Lua 5.5 source files under `~/Downloads/lua-lua-a5522f0`.

Choose the next incomplete step from `docs/PROGRESS.md`. Keep the change small, add tests, run the narrowest useful verification command, update `docs/PROGRESS.md`, and commit with a conventional commit message. Do not implement unrelated future steps.
```

## Emergency `/goal` Prompt for Cleanup

Use this when the repository has drifted, tests are broken, or changes became too large.

```text
/goal
Stabilize the Elara repository without adding new features.

Read `docs/ARCHITECTURE.md`, `docs/MILESTONES.md`, `docs/PROGRESS.md`, and the cleanup rules in `docs/CODEX_GOAL.md`. If behavior correctness is involved, inspect the relevant Lua 5.5 source files under `~/Downloads/lua-lua-a5522f0`.

Focus only on restoring a clean, reviewable state: run formatting and the smallest useful failing test, fix compilation or test failures, split oversized files only when necessary, update `docs/PROGRESS.md`, and commit with a conventional cleanup message.
```

## Commit Rules

All commits should use conventional commit messages.

Format:

```text
<type>(<scope>): <description>
```

Allowed common types:

```text
feat      new behavior
fix       bug fix
perf      performance improvement
refactor  structural change without behavior change
test      test-only or test infrastructure change
docs      documentation-only change
chore     repository maintenance
bench     benchmark-only change
ci        CI configuration
```

Examples:

```text
chore(workspace): bootstrap elara workspace
feat(core): add lua value primitives
feat(bytecode): add verifier
feat(interp): execute primitive arithmetic
test(syntax): add parser snapshots
perf(interp): optimize table array fast path
docs(progress): update current milestone status
```

Bad commit shapes:

```text
feat: implement VM
wip
changes
massive update
fix stuff
```

## Step Size Rules

A step is acceptable when:

- It can be reviewed without reading unrelated subsystems.
- It has a clear verification command.
- It updates or adds tests when behavior changes.
- It does not require more than a small number of files unless the step is a structural bootstrap.
- It does not leave the workspace in a confusing partial state.

A step is too large when:

- It changes parser, compiler, runtime, stdlib, and API at the same time.
- It produces huge files.
- It requires future work to understand whether it is correct.
- It cannot be tested independently.
- It mixes formatting churn with feature work.

If a milestone step feels too large, split it into smaller internal steps and record that in `docs/PROGRESS.md`.

## File Size and Module Structure Rules

Hard rule:

```text
No single Rust source file should exceed 1000 lines.
```

Preferred rule:

```text
Keep most source files below 600 lines.
```

When a file grows too large, split by responsibility. Examples:

```text
elara-core/src/value.rs
elara-core/src/value/tag.rs
elara-core/src/value/convert.rs
elara-core/src/gc/header.rs
elara-core/src/gc/arena.rs
elara-core/src/gc/trace.rs
elara-syntax/src/lexer.rs
elara-syntax/src/parser/expr.rs
elara-syntax/src/parser/stmt.rs
elara-bytecode/src/instr.rs
elara-bytecode/src/verify.rs
elara-interp/src/dispatch.rs
elara-interp/src/ops/table.rs
elara-interp/src/ops/call.rs
```

Do not create a single `vm.rs` that absorbs the whole runtime.

## Architecture Guardrails

Keep these boundaries:

- `elara-core` does not depend on syntax, compiler, stdlib, or JIT.
- `elara-syntax` does not execute code.
- `elara-compiler` emits bytecode and diagnostics; it does not interpret.
- `elara-bytecode` defines and verifies bytecode; it does not parse Lua.
- `elara-interp` executes verified bytecode; it does not know source parser details.
- `elara-jit` consumes verified bytecode; it does not parse Lua.
- `elara-api` exposes safe embedding abstractions and hides raw runtime internals.
- `elara-capi` is optional and targets only the current Lua version.

If a change violates these boundaries, stop and update the architecture only if the new design is intentional.

## Verification Policy

For every step, run the narrowest command that proves the change.

Examples:

```bash
cargo test -p elara-core value
cargo test -p elara-syntax lexer
cargo test -p elara-bytecode verifier
cargo test -p elara-interp arithmetic
cargo test --workspace --features jit jit_equivalence
```

Before large milestone transitions, run broader checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Before release work, run all available checks:

```bash
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
cargo bench -p elara-bench
```

If a command cannot run because the workspace is not ready, document the exact reason in `docs/PROGRESS.md` and run the closest meaningful command.

## Progress Document Update Rules

`docs/PROGRESS.md` must answer:

1. What milestone is current?
2. What step is current?
3. What is complete now?
4. What remains?
5. What verification command was last run?
6. What is the next recommended step?
7. What known gaps or risks should the next agent understand?

Do not turn `PROGRESS.md` into a changelog. Replace stale status with current status.

Good progress update:

```text
Current milestone: M2 Runtime Value Model and Basic GC Skeleton
Current step: M2.2 Add GC pointer and object header types
Completed: M2.1 Value primitives with tests.
Remaining gaps: GC pointer wrappers and allocation list are not implemented yet.
Last verification: cargo test -p elara-core value
Next step: implement GcHeader, GcKind, GcColor, and internal GcRef<T>.
```

Bad progress update:

```text
2026-06-05: did some stuff
2026-06-06: did more stuff
2026-06-07: many fixes
```

## Refactoring Policy

Refactoring is allowed when it keeps the next feature clean. It should usually be separate from feature commits.

Acceptable:

```text
refactor(core): split value conversions into module
feat(core): add string interning
```

Avoid:

```text
feat(core): add strings and rewrite half the workspace
```

## Unsafe Policy for Codex

Before adding unsafe code, ask whether the same step can be done safely without harming the architecture.

If unsafe is required:

- Keep the unsafe block small.
- Put unsafe in internal modules, not public API surfaces.
- Add `// SAFETY:` comments explaining preconditions.
- Add tests through safe APIs.
- Do not let JIT or FFI calls bypass GC rooting rules.

## JIT Policy for Codex

JIT work starts only after interpreter semantics and bytecode verification are stable enough.

JIT rules:

- JIT is optional behind a feature flag.
- Interpreter remains the semantic reference.
- Generated code must return to interpreter on unsupported cases.
- Safepoints must sync live Lua values to the VM stack before allocation or calls.
- Debug hooks, coroutine yield, and complex metamethod paths may force interpretation.
- All JIT tests must compare against interpreter results.

## Definition of Done for the Whole Project

Elara reaches the intended target when:

- The main branch targets the current stable Lua version only.
- Lua source can be parsed, compiled, verified, and executed.
- Core language semantics, tables, closures, coroutines, errors, and standard libraries are implemented.
- Rust embedding API supports typed values, native functions, tables, registry keys, and userdata.
- Interpreter performance is measured and competitive with reference C Lua on representative workloads.
- Cranelift JIT is optional, tested, and beneficial for hot supported code.
- Conformance and differential tests describe correctness against official Lua.
- Architecture remains layered and maintainable.
- Current limitations are documented in `docs/PROGRESS.md` or release notes.
