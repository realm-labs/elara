# Elara Progress

Status: Rolling current-state document  
Last updated: 2026-06-06  
Current target: latest stable Lua, currently Lua 5.5 / Lua 5.5.0  
Current milestone: M0 Repository Bootstrap  
Current step: M0.2 Add lint, formatting, and CI configuration

This document is for orientation. It is not a changelog. When work progresses,
replace stale status with the current state instead of appending history.

## Current Snapshot

Elara has a Cargo workspace skeleton with project documentation and empty crate
boundaries. Runtime, parser, compiler, bytecode, interpreter, API, JIT, C API,
conformance, and benchmark implementation work has not started.

Current state:

```text
Completed:
  - Project name selected: Elara.
  - Direction selected: only support the latest stable Lua version on main.
  - Architecture direction selected: Rust-native VM, high-level API, optional Cranelift JIT.
  - Documentation drafts prepared.
  - M0.1 Create workspace skeleton.

In progress:
  - M0.2 Add lint, formatting, and CI configuration.

Not started:
  - M0.3 Define crate-level module policies.
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

### Current Step: M0.2 Add lint, formatting, and CI configuration

Expected deliverables:

- Shared lint configuration in workspace `Cargo.toml`.
- `.rustfmt.toml` if needed.
- GitHub Actions or equivalent CI for fmt, clippy, and test.
- Basic `xtask` placeholder if desired.

Recommended verification:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Recommended commit:

```text
chore(ci): add workspace quality gates
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

- Root `elara` crate exists.
- Workspace uses placeholder member crates for the architecture-defined layers.
- Each placeholder crate has a minimal manifest and `src/lib.rs`.
- Root README describes project positioning and workspace layout.

## Remaining Gaps

### Immediate Gaps for M0

- Add lint and CI configuration in M0.2.
- Add crate boundary docs in M0.3.

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

M0.1 verification passed:

```bash
cargo metadata --format-version 1
cargo fmt --all -- --check
```

## Next Recommended Action

Implement M0.2 from `docs/MILESTONES.md`:

1. Add shared workspace lint configuration.
2. Add formatting configuration if needed.
3. Add CI for fmt, clippy, and tests.
4. Run the recommended M0.2 verification commands.
5. Update this progress document.
6. Commit with:

```text
chore(ci): add workspace quality gates
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
| Workspace | Complete | Root package and placeholder member crates exist. |
| CI and lints | Not started | Current step. |
| Crate boundary docs | Not started | Planned for M0.3. |
| Core runtime | Not started | Starts M2. |
| Parser | Not started | Starts M4. |
| Compiler | Not started | Starts M5. |
| Interpreter | Not started | Starts M6. |
| Rust API | Not started | Starts M12. |
| JIT | Not started | Starts M16. |
| C API | Not started | Starts M19, optional/current-version only. |
| Benchmarks | Not started | Starts M15. |
