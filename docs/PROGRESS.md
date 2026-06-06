# Elara Progress

Status: Rolling current-state document  
Last updated: 2026-06-06  
Current target: latest stable Lua, currently Lua 5.5 / Lua 5.5.0  
Current milestone: M9 Tables, Metamethods, and Globals
Current step: M9.3 Implement metatable and metamethod dispatch

This document is for orientation. It is not a changelog. When work progresses,
replace stale status with the current state instead of appending history.

## Current Snapshot

Elara has a Cargo workspace skeleton with project documentation, quality gates,
documented crate boundaries, and a single current Lua language target in
`elara-core`. Core source span and diagnostic primitives are available, and the
test fixture/conformance/differential directory layout exists with a snapshot
baseline. Primitive runtime values, the basic GC skeleton, Lua strings with
short-string interning, table array/hash storage with metadata versioning, Lua
tokenization, expression parsing, statement parsing, and parser snapshot/error
coverage are implemented. The initial bytecode prototype, instruction encoding,
opcode set, constant pool, metadata placeholders, builder, and disassembler are
implemented, along with initial bytecode verification and simple expression
codegen. VM/thread stack primitives and primitive arithmetic bytecode execution
are implemented, and simple source chunks can be evaluated through the compile
and interpreter path. Local variables and assignment basics are implemented.
Simple function Protos and zero-argument Lua calls are implemented. Closures,
upvalues, and captured outer local reads are implemented. Anonymous vararg
functions can receive call arguments and lower `...` for the first requested
value. Fixed-count Lua calls can receive multiple return values. Named vararg
tables are compiled to `VARARG_TABLE` and executed through runtime-owned table
storage. Recursive function self-references compile and evaluate through shared
runtime closure storage. Conditional branches compile and execute through
`TEST`/`JMP` bytecode. `while`, `repeat`, and `break` compile and execute
through branch bytecode. Numeric for loops compile and execute for integer and
float control values with positive and negative steps. Generic for loops compile
and execute through the iterator-call protocol. Table constructors compile and
execute with array, record, and keyed fields. Raw table access compiles and
executes for bracket and field syntax, generic table get/set bytecode, integer
index fast paths, hash keys, and nil assignment clearing. Primitive runtime
table storage is centralized and can store metatable links for upcoming
metamethod dispatch. Table-valued `__index` and `__newindex` chains work in the
primitive runtime table slow path. Globals, function-valued and operator
metamethod dispatch, full API, JIT, C API, conformance, and benchmark
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
  - M1.4 Add snapshot testing baseline.
  - M2.1 Implement Value and primitive conversions.
  - M2.2 Add GC pointer and object header types.
  - M2.3 Implement basic arena/list allocator.
  - M2.4 Implement stop-the-world mark-sweep MVP.
  - M3.1 Implement Lua strings and interning.
  - M3.2 Implement table array storage.
  - M3.3 Implement table hash storage.
  - M3.4 Add metatable field and table versioning.
  - M4.1 Implement lexer for tokens and literals.
  - M4.2 Implement expression parser.
  - M4.3 Implement statement parser.
  - M4.4 Implement parser snapshots and error tests.
  - M5.1 Define Proto, Instr, and opcode encoding.
  - M5.2 Add bytecode builder and disassembler.
  - M5.3 Add bytecode verifier.
  - M5.4 Compile constants and arithmetic expressions.
  - M6.1 Add VM state and thread stack.
  - M6.2 Implement interpreter loop for constants and arithmetic.
  - M6.3 Connect source compile and eval path.
  - M7.1 Implement local variables and assignment.
  - M7.2 Implement function Protos and Lua calls.
  - M7.3 Implement closures and upvalues.
  - M7.4 Implement varargs and named vararg table.
  - M7 exit criteria validation.
  - M8.1 Implement conditional branches.
  - M8.2 Implement while and repeat loops.
  - M8.3 Implement numeric for loops.
  - M8.4 Implement generic for loops.
  - M8 exit criteria validation.
  - M9.1 Compile and execute table constructors.
  - M9.2 Implement table get/set bytecode.

In progress:
  - M9.3 Implement metatable and metamethod dispatch.
    - Runtime table storage is centralized with metatable sidecar links.
    - Table-valued `__index` and `__newindex` chains work in the runtime table
      slow path.
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

### Completed Step: M3.3 Implement table hash storage

Delivered:

- Hash part with canonical Lua `Value` keys.
- Numeric key canonicalization for integer-like floats.
- Nil and NaN key rejection.
- Tests for string, integer, float, and boolean keys.

### Completed Step: M3.4 Add metatable field and table versioning

Delivered:

- Optional metatable pointer.
- Meta flags placeholder.
- Version bump on structural mutations.
- Tests for version changes.

M3 is complete.

### Completed Step: M4.1 Implement lexer for tokens and literals

Delivered:

- Keywords, identifiers, punctuation, and operators.
- Integer, float, and string literal tokenization.
- Comments and whitespace handling.
- Error diagnostics for invalid tokens.

### Completed Step: M4.2 Implement expression parser

Delivered:

- Operator precedence.
- Unary and binary operators.
- Function call expressions.
- Table constructors.
- Vararg expression.

### Completed Step: M4.3 Implement statement parser

Delivered:

- Assignment.
- Local declarations.
- Global declarations for current Lua.
- Function declarations.
- If/while/repeat/for.
- Return/break/goto/labels where current Lua supports them.

### Completed Step: M4.4 Implement parser snapshots and error tests

Delivered:

- Snapshot tests for representative chunks.
- Error tests for malformed syntax.
- AST pretty debug output.

M4 is complete.

### Completed Step: M5.1 Define Proto, Instr, and opcode encoding

Delivered:

- `Proto`, `Instr`, and `Op`.
- Constant pool.
- Upvalue descriptors placeholder.
- Debug info placeholder.

### Completed Step: M5.2 Add bytecode builder and disassembler

Delivered:

- Builder API.
- Human-readable disassembly.
- Tests for simple instruction sequences.

### Completed Step: M5.3 Add bytecode verifier

Delivered:

- Register bounds checks.
- Constant bounds checks.
- Jump target checks.
- Basic call/return layout checks.

### Completed Step: M5.4 Compile constants and arithmetic expressions

Delivered:

- Compile literal expressions.
- Compile unary and binary arithmetic expressions.
- Emit constants.
- Verify generated bytecode.

M5 is complete.

### Completed Step: M6.1 Add VM state and thread stack

Delivered:

- `Vm` or `Runtime` state.
- `LuaThread` stack.
- Basic call frame.
- Stack push/pop helpers.

### Completed Step: M6.2 Implement interpreter loop for constants and arithmetic

Delivered:

- `LOAD_*`, `ADD`, `SUB`, `MUL`, `DIV`, `IDIV`, `RETURN`.
- Primitive runtime errors.
- Tests executing simple Protos.

### Completed Step: M6.3 Connect source compile and eval path

Delivered:

- Public internal function: source -> Proto -> interpreter.
- Test `return 42` from source.
- Test arithmetic source chunks.

M6 is complete.

### Completed Step: M7.1 Implement local variables and assignment

Delivered:

- Scope resolution.
- Register allocation for locals.
- Multiple assignment basics.
- Tests for local variable behavior.

### Completed Step: M7.2 Implement function Protos and Lua calls

Delivered:

- Function AST lowering.
- Nested Proto emission.
- `CALL` and `RETURN` basics.
- Tests for simple function calls.

### Completed Step: M7.3 Implement closures and upvalues

Delivered:

- Upvalue analysis.
- Open/closed upvalue runtime.
- Closure bytecode.
- Tests for nested functions.

### Completed Step: M7.4 Implement varargs and named vararg table

Delivered:

- Vararg function handling.
- `...` lowering.
- Named vararg table support for current Lua.
- Multiple return tests.
- Anonymous vararg functions can be compiled with `Proto::is_vararg`.
- `...` lowers to `VARARG` in anonymous vararg functions.
- Lua call arguments are placed in call registers and passed to child Protos.
- The primitive interpreter executes simple `VARARG` reads with nil fill.
- Fixed-count `CALL` results write all requested return registers with nil fill.
- Open-ended `VARARG`, `CALL`, and `RETURN` propagate all available results.
- Named vararg tables lower to `VARARG_TABLE`.
- The primitive interpreter materializes named vararg tables into runtime-owned table storage.

Recommended verification:

```bash
cargo test -p elara-interp varargs
```

Recommended commit:

```text
feat(runtime): support varargs
```

### Completed Step: M7 Exit Criteria Validation

Delivered:

- Function bodies can capture their own local function binding.
- Runtime closure storage is shared across nested executions so closure values
  remain callable across frames.
- Recursive self-reference works through the source compile/eval path.
- Recursive self-reference exit criteria are validated through source eval.

M7 is complete.

### Completed Step: M8.1 Implement conditional branches

Delivered:

- `TEST` and `JMP` execute in the primitive interpreter.
- The bytecode builder can patch forward `JMP` offsets.
- The verifier accepts jumps to the end-of-code boundary.
- The simple compiler lowers `if`, `elseif`, and `else` blocks.
- Source eval executes simple `if/else` chunks.

### Completed Step: M8.2 Implement while and repeat loops

Delivered:

- The simple compiler lowers `while` loops with false-condition exits and back
  jumps.
- The simple compiler lowers `repeat` loops with post-body condition checks.
- `break` statements inside loops patch to the loop exit.
- `break` outside a loop reports a compile diagnostic.
- Source eval executes simple `while`/`break` and `repeat` chunks.

### Completed Step: M8.3 Implement numeric for loops

Delivered:

- The simple compiler lowers numeric `for` loops to `FOR_PREP`/`FOR_LOOP`.
- The bytecode verifier checks the three-register numeric loop control range.
- The primitive interpreter executes integer numeric loops with positive and
  negative steps.
- The primitive interpreter executes float numeric loops.
- Zero numeric-for steps report a runtime error.
- Source eval executes simple numeric `for` chunks with positive and negative
  steps.

### Completed Step: M8.4 Implement generic for loops

Delivered:

- The simple compiler lowers generic `for` loops to `TFOR_PREP`, `TFOR_CALL`,
  and `TFOR_LOOP`.
- The bytecode verifier checks generic-for state and result register ranges.
- The primitive interpreter executes iterator calls with state/control values.
- Generic loops enter the body only when the first iterator result is non-nil.
- Source eval executes simple generic `for` chunks using iterator functions.

M8 is complete.

### Completed Step: M9.1 Compile and execute table constructors

Delivered:

- Bytecode prototypes carry string constants for record-field keys.
- The simple compiler lowers table constructors to `NEW_TABLE` and `SET_TABLE`.
- Array fields use consecutive integer keys starting at 1.
- Named fields use interned short-string keys.
- Keyed fields compile and write their explicit key expressions.
- The primitive interpreter executes `NEW_TABLE`, `LOAD_STRING`, and
  constructor `SET_TABLE` writes into runtime-owned table storage.
- Source eval can return table constructor values.

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
- Table hash storage and numeric key canonicalization are available.
- Table metatable metadata, meta flags, and structural versioning are available.
- Lua tokenization with spans and lexical diagnostics is available in `elara-syntax`.
- Lua expression AST and precedence parsing are available in `elara-syntax`.
- Lua statement AST and block parsing are available in `elara-syntax`.
- Parser snapshots and malformed syntax diagnostics are covered.
- Bytecode opcode, instruction encoding, prototype, constant pool, upvalue descriptor, debug placeholder, builder, disassembler, and verifier are available.
- Simple return-expression bytecode codegen is available in `elara-compiler`.
- VM state, Lua thread stack, call frames, and stack helpers are available in `elara-core`.
- Primitive bytecode execution for constants, arithmetic, and return values is available in `elara-interp`.
- Simple source chunks can be compiled and evaluated through `elara-api`.
- Local variable reads/writes and assignment basics are available in the compiler/interpreter path.
- Simple nested Protos and zero-argument Lua calls are available.
- Captured outer local reads work through upvalue descriptors and runtime closure captures.
- Anonymous vararg argument passing and first-value `...` lowering work in the compiler/interpreter path.
- Fixed-count multiple call results work in the primitive interpreter.
- Open-ended vararg call/return results work in the primitive interpreter.
- Named vararg tables compile and execute through runtime-owned table storage.
- Recursive self-reference works through the compiler/interpreter/API path.
- Conditional branches work through the compiler/interpreter/API path.
- `while`, `repeat`, and `break` work through the compiler/interpreter/API path.
- Numeric `for` loops work through the compiler/interpreter/API path.
- Generic `for` loops work through the compiler/interpreter/API path.
- Table constructors work through the compiler/interpreter/API path.
- Raw table access works through the parser/compiler/interpreter/API path.
- Primitive runtime table storage has a centralized owner with metatable links.
- Table-valued `__index` and `__newindex` chains work in runtime table helpers.

## Remaining Gaps

### Immediate Gaps for M9

- Add function-valued `__index` and `__newindex` calls.
- Add arithmetic, comparison, `__len`, `__call`, and `__concat` metamethod dispatch.

### Product Gaps

Major implementation work is still pending:

- Global declaration behavior.
- Metamethod dispatch.
- Standard library.
- Rust API.
- Cranelift JIT.
- Optional C API.
- Conformance tests.
- Differential tests.
- Benchmarks.

## Last Verification

M9.3 table-valued metamethod verification passed:

```bash
cargo test -p elara-interp metamethods
cargo test -p elara-interp table_access
cargo fmt --all -- --check
cargo clippy -p elara-interp --all-targets -- -D warnings
```

## Next Recommended Action

Implement M9.3 function-valued `__index` and `__newindex` calls, using Lua 5.5
`luaV_finishget`, `luaV_finishset`, and `luaT_callTM/res` behavior as
references.

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
| Core runtime | Initial foundation complete | Value, GC, string, and table foundations are implemented. |
| Value primitives | Complete | Nil, bool, integer, and float values are implemented. |
| GC headers | Complete | Headers, colors, kinds, and typed refs are implemented. |
| GC allocation | Complete | Arena allocation list, stats, roots, and drop cleanup are implemented. |
| Mark-sweep GC | Complete | Root marking and allocation-list sweeping are implemented for tests. |
| Strings | Complete | Short strings, long strings, and interning are implemented. |
| Table array | Complete | Raw 1-based array get/set and nil clearing are implemented. |
| Table hash | Complete | Hash storage and numeric key canonicalization are implemented. |
| Table metadata | Complete | Metatable pointer, meta flags, and structural versioning are implemented. |
| Lexer | Complete | Lua 5.5 tokens, literals, comments, and lexical diagnostics are implemented. |
| Expression parser | Complete | Expression AST, precedence parsing, calls, table constructors, and varargs are implemented. |
| Statement parser | Complete | Declarations, assignments, control flow, function declarations, labels, and returns are implemented. |
| Parser snapshots | Complete | Representative AST and malformed syntax diagnostic snapshots are implemented. |
| Bytecode model | Initial model complete | Proto, instruction encoding, opcode set, constants, upvalues, debug placeholders, builder, disassembler, and verifier are implemented. |
| Compiler | Initial MVP complete | Simple return-expression codegen emits verified bytecode. |
| VM/thread stack | Complete | VM state, Lua thread stack, call frames, and stack helpers are implemented. |
| Interpreter | Initial MVP complete | Simple compiled source chunks can execute constants, arithmetic, and returns. |
| Variables/scopes | Complete | Local variables, assignment basics, simple calls, captured outer local reads, anonymous and named varargs, multiple call results, and recursive self-reference are implemented. |
| Control flow | Complete | Conditional branches, `while`, `repeat`, `break`, numeric `for`, and generic `for` execute through bytecode. |
| Tables/globals/metamethods | In progress | Table constructors, raw table access, and table-valued `__index`/`__newindex` dispatch execute. |
| Rust API | Not started | Starts M12. |
| JIT | Not started | Starts M16. |
| C API | Not started | Starts M19, optional/current-version only. |
| Benchmarks | Not started | Starts M15. |
