# Elara Progress

Status: Rolling current-state document  
Last updated: 2026-06-06  
Current target: latest stable Lua, currently Lua 5.5 / Lua 5.5.0  
Current milestone: M3 Strings and Tables  
Current step: M3.3 Implement table hash storage

This document is for orientation. It is not a changelog. When work progresses,
replace stale status with the current state instead of appending history.

## Current Snapshot

Elara has a Cargo workspace skeleton with project documentation, quality gates,
documented crate boundaries, and a single current Lua language target in
`elara-core`. Core source span and diagnostic primitives are available, and the
test fixture/conformance/differential directory layout exists with a snapshot
baseline. Primitive runtime values, the basic GC skeleton, and Lua strings with
short-string interning are implemented. Table array storage is implemented.
Table hash storage, parser, compiler, bytecode, interpreter, API, JIT, C API,
conformance, and benchmark implementation work has not started.

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
  - M1.4 Add snapshot testing baseline.
  - M2.1 Implement Value and primitive conversions.
  - M2.2 Add GC pointer and object header types.
  - M2.3 Implement basic arena/list allocator.
  - M2.4 Implement stop-the-world mark-sweep MVP.
  - M3.1 Implement Lua strings and interning.
  - M3.2 Implement table array storage.

Not started:
  - M3.3 Implement table hash storage.
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

### Completed Step: M1.4 Add snapshot testing baseline

Delivered:

- Snapshot helper for AST, bytecode, and diagnostics paths.
- Diagnostics snapshot formatting helper.
- First trivial fixture: `return 42`.
- Baseline diagnostics snapshot for `return 42`.

M1 is complete.

### Completed Step: M2.1 Implement Value and primitive conversions

Delivered:

- `Value` and `ValueTag`.
- Nil, boolean, integer, and float constructors and accessors.
- Primitive value equality, including integer/float numeric equality.
- Numeric helpers for float and exact integer conversion.

### Completed Step: M2.2 Add GC pointer and object header types

Delivered:

- `GcHeader`, `GcKind`, and `GcColor`.
- `GcObject` trait for GC-managed payloads.
- `GcRef<T>` typed internal reference wrapper.
- Unsafe pointer helpers are localized and documented with SAFETY comments.
- No public raw GC pointer accessor was added.

### Completed Step: M2.3 Implement basic arena/list allocator

Delivered:

- Runtime-owned `GcArena` allocation list.
- `GcStats` allocation/root counters.
- `GcRoot` placeholder root handles.
- Drop cleanup for arena-owned objects.

### Completed Step: M2.4 Implement stop-the-world mark-sweep MVP

Delivered:

- Stop-the-world collection entry point on `GcArena`.
- Mark phase over placeholder roots.
- Sweep phase over the allocation list.
- Tests for reachable and unreachable object collection.

M2 is complete.

### Completed Step: M3.1 Implement Lua strings and interning

Delivered:

- `ShortString` and `LongString` GC objects.
- Deterministic byte hashing for Lua strings.
- `StringInterner` for rooted interned short strings.
- String equality and hashing tests.

### Completed Step: M3.2 Implement table array storage

Delivered:

- `Table` GC object with array part storage.
- Raw 1-based integer array get/set helpers.
- Nil assignment clears slots and trims trailing nils.
- Array growth tests.

### Current Step: M3.3 Implement table hash storage

Expected deliverables:

- Hash part with Lua `Value` keys.
- Numeric key canonicalization.
- Rehash logic.
- Tests for string, integer, float, and boolean keys.

Recommended verification:

```bash
cargo test -p elara-core table_hash
```

Recommended commit:

```text
feat(core): add table hash storage
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
- Snapshot helpers and a `return 42` fixture baseline are present.
- Primitive Lua values are available in `elara-core`.
- GC object headers and typed references are available in `elara-core`.
- Basic GC allocation list, stats, roots, and drop cleanup are available.
- Stop-the-world mark-sweep MVP is available under tests.
- Lua short strings, long strings, and short-string interning are available.
- Table array storage is available in `elara-core`.

## Remaining Gaps

### Immediate Gaps for M3

- Add table hash storage and numeric key canonicalization.

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

M3.2 verification passed:

```bash
cargo fmt --all -- --check
cargo clippy -p elara-core --all-targets -- -D warnings
cargo test -p elara-core table_array
```

## Next Recommended Action

Implement M3.3 from `docs/MILESTONES.md`:

1. Add table hash storage with Lua `Value` keys.
2. Add numeric key canonicalization.
3. Add tests for string, integer, float, and boolean keys.
4. Run `cargo test -p elara-core table_hash`.
5. Update this progress document.
6. Commit with:

```text
feat(core): add table hash storage
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
| Snapshots | Complete | Snapshot helper and `return 42` diagnostics baseline exist. |
| Core runtime | Not started | Current milestone. |
| Value primitives | Complete | Nil, bool, integer, and float values are implemented. |
| GC headers | Complete | Headers, colors, kinds, and typed refs are implemented. |
| GC allocation | Complete | Arena allocation list, stats, roots, and drop cleanup are implemented. |
| Mark-sweep GC | Complete | Root marking and allocation-list sweeping are implemented for tests. |
| Strings | Complete | Short strings, long strings, and interning are implemented. |
| Table array | Complete | Raw 1-based array get/set and nil clearing are implemented. |
| Table hash | Not started | Current step. |
| Parser | Not started | Starts M4. |
| Compiler | Not started | Starts M5. |
| Interpreter | Not started | Starts M6. |
| Rust API | Not started | Starts M12. |
| JIT | Not started | Starts M16. |
| C API | Not started | Starts M19, optional/current-version only. |
| Benchmarks | Not started | Starts M15. |
