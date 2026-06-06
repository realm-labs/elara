# Elara Progress

Status: Rolling current-state document  
Last updated: 2026-06-06  
Current target: latest stable Lua, currently Lua 5.5 / Lua 5.5.0  
Current milestone: M1 Language Specification and Test Harness  
Current step: M1.4 Add snapshot testing baseline

This document is for orientation. It is not a changelog. When work progresses,
replace stale status with the current state instead of appending history.

## Current Snapshot

Elara has a Cargo workspace skeleton with project documentation, quality gates,
documented crate boundaries, and a single current Lua language target in
`elara-core`. Core source span and diagnostic primitives are available, and the
test fixture/conformance/differential directory layout exists. Runtime, parser,
compiler, bytecode, interpreter, API, JIT, C API, conformance, and benchmark
implementation work has not started.

Current state:

```text
Completed:
  - Project name selected: Elara.
  - Direction selected: only support the latest stable Lua version on main.
  - Architecture direction selected: Rust-native VM, high-level API, optional Cranelift JIT.
  - Documentation drafts prepared.
  - M0.1 Create workspace skeleton.
  - M0.2 Add lint, formatting, and CI configuration.
  - M0.3 Define crate-level module policies.
  - M1.1 Add spec module.
  - M1.2 Add diagnostic and source span primitives.
  - M1.3 Add test fixture layout.

Not started:
  - M1.4 Add snapshot testing baseline.
  - Runtime core.
  - Parser.
  - Compiler.
  - Bytecode.
  - Interpreter.
  - Standard library.
  - Rust API.
  - JIT.
  - C API.
  - Conformance and benchmark harnesses.
```

## Current Milestone Details

### M0 Repository Bootstrap

Goal: create a structured Rust workspace for Elara with project documentation,
CI-ready defaults, and empty crates.

### Completed Step: M0.1 Create workspace skeleton

Delivered:

- Root `Cargo.toml` workspace.
- Root `README.md`.
- `docs/ARCHITECTURE.md`.
- `docs/MILESTONES.md`.
- `docs/CODEX_GOAL.md`.
- `docs/PROGRESS.md`.
- `crates/` directory.
- Placeholder crates:
  - `elara-core`
  - `elara-syntax`
  - `elara-compiler`
  - `elara-bytecode`
  - `elara-interp`
  - `elara-stdlib`
  - `elara-api`
  - `elara-jit`
  - `elara-capi`
  - `elara-test`
  - `elara-bench`

### Completed Step: M0.2 Add lint, formatting, and CI configuration

Delivered:

- Shared lint configuration in workspace `Cargo.toml`.
- Member crates inherit workspace lint settings.
- Root `.rustfmt.toml`.
- GitHub Actions CI for fmt, clippy, and tests.

### Completed Step: M0.3 Define crate-level module policies

Delivered:

- Each workspace crate has `lib.rs` module-level boundary docs.
- `elara` facade crate exists under `crates/elara`.
- `elara` re-exports only the stable public API placeholder module from `elara-api`.

M0 is complete.

### Completed Step: M1.1 Add spec module

Delivered:

- `elara-core` exposes `LuaVersion`, `LuaSpec`, `LUA_VERSION`, and `LUA_SPEC`.
- The current target is Lua 5.5 / Lua 5.5.0.
- No old-version `LuaDialect` enum or compatibility flag was added.

### Completed Step: M1.2 Add diagnostic and source span primitives

Delivered:

- `SourceId`, `Span`, and `Spanned<T>`.
- `DiagnosticSeverity`, `DiagnosticLabel`, and `Diagnostic`.
- Display formatting and focused tests for source spans and diagnostics.

### Completed Step: M1.3 Add test fixture layout

Delivered:

- `tests/fixtures/pass/`.
- `tests/fixtures/fail/`.
- `tests/conformance/`.
- `tests/differential/`.
- Placeholder harness docs for fixture, conformance, and differential areas.

### Current Step: M1.4 Add snapshot testing baseline

Expected deliverables:

- Snapshot helper for AST/bytecode/diagnostics.
- First trivial fixture: `return 42`.

Recommended verification:

```bash
cargo test --workspace snapshots
```

Recommended commit:

```text
test(harness): add snapshot baseline
```

## Completed Content

### Planning Decisions

- Project name: Elara.
- Main branch target: current stable Lua only.
- Old Lua versions: not implemented through compatibility flags.
- Historical support policy: old versions should be preserved through tags or maintenance branches, not mainline dialect branching.
- Execution model: source -> AST -> HIR -> internal bytecode -> interpreter/JIT.
- JIT backend: Cranelift, optional feature, method JIT first.
- API direction: Rust-first embedding API with typed conversions, native functions, tables, registry, and userdata.
- GC direction: custom tracing GC, starting with stop-the-world mark-sweep, evolving to incremental collection.

### Workspace Bootstrap

- The repository root is a virtual Cargo workspace.
- Workspace uses placeholder member crates for the architecture-defined layers.
- `crates/elara` is the public facade crate and depends only on `elara-api`.
- Each placeholder crate has a minimal manifest and `src/lib.rs`.
- Root README describes project positioning and workspace layout.
- Workspace quality gates are configured through Cargo lints, rustfmt, and GitHub Actions.
- Crate-level module docs describe boundary policies.
- The current Lua target is declared once in `elara-core`.
- Core source span and diagnostic primitives are available.
- Test fixture, conformance, and differential directories are present.

## Remaining Gaps

### Immediate Gaps for M1

- Add snapshot helper and baseline fixture.

### Product Gaps

All implementation work is still pending:

- Value representation.
- GC object model.
- String interning.
- Table array/hash storage.
- Parser and diagnostics.
- Bytecode format and verifier.
- Compiler.
- Interpreter.
- Standard library.
- Rust API.
- Cranelift JIT.
- Optional C API.
- Conformance tests.
- Differential tests.
- Benchmarks.

## Last Verification

M1.3 verification passed:

```bash
cargo test --workspace
```

## Next Recommended Action

Implement M1.4 from `docs/MILESTONES.md`:

1. Add a snapshot helper for diagnostics and future AST/bytecode snapshots.
2. Add the first trivial fixture: `return 42`.
3. Run `cargo test --workspace snapshots`.
4. Update this progress document.
5. Commit with:

```text
test(harness): add snapshot baseline
```

## Current Risk Notes

- Do not introduce old Lua dialect infrastructure during bootstrap.
- Do not over-design JIT before bytecode and interpreter semantics exist.
- Keep `elara-core` independent from parser, compiler, stdlib, and JIT.
- Keep crate files small and focused as implementation begins.
- Keep `docs/PROGRESS.md` concise and current; do not append historical diary entries.

## Status Table

| Area | Status | Notes |
|---|---|---|
| Project naming | Complete | Elara selected. |
| Language target | Complete | Latest stable Lua only. |
| Architecture docs | Drafted | Present in `docs/`. |
| Milestone plan | Drafted | Present in `docs/`. |
| Codex goal | Drafted | Present in `docs/`. |
| Workspace | Complete | Virtual workspace and placeholder member crates exist. |
| CI and lints | Complete | Cargo lints, rustfmt, and GitHub Actions are configured. |
| Crate boundary docs | Complete | Module docs describe crate responsibilities and dependencies. |
| Lua spec | Complete | `elara-core` declares Lua 5.5 / 5.5.0 as the only current target. |
| Diagnostics | Complete | Source spans and structured diagnostics are in `elara-core`. |
| Test fixtures | Complete | Fixture, conformance, and differential directories are present. |
| Snapshots | Not started | Current step. |
| Core runtime | Not started | Starts M2. |
| Parser | Not started | Starts M4. |
| Compiler | Not started | Starts M5. |
| Interpreter | Not started | Starts M6. |
| Rust API | Not started | Starts M12. |
| JIT | Not started | Starts M16. |
| C API | Not started | Starts M19, optional/current-version only. |
| Benchmarks | Not started | Starts M15. |
