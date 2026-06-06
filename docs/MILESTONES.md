# Elara Implementation Plan

Status: Draft 1  
Purpose: step-by-step implementation plan for Codex or another coding agent.  
Completion target: after all required milestones are done, Elara should be a complete latest-Lua VM in Rust with a high-level Rust API, optimized interpreter, and Cranelift JIT.

## 0. Execution Rules for Milestones

Each milestone is divided into verifiable steps. Do not implement an entire milestone as one huge change. Each step should end with:

1. Code or documentation changes that are small enough to review.
2. Formatting and tests for the touched area.
3. A conventional commit.
4. An update to `docs/PROGRESS.md`.

Suggested verification commands should be adapted as the workspace evolves.

Base commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If a crate is not ready for full workspace checks, run the narrowest meaningful command and record the gap in `docs/PROGRESS.md`.

Conventional commit examples:

```text
chore(workspace): bootstrap cargo workspace
feat(bytecode): add proto and instruction definitions
test(parser): add expression precedence snapshots
fix(table): canonicalize integer-like float keys
docs(progress): update current milestone status
```

## Milestone M0: Repository Bootstrap

Goal: create a structured Rust workspace for Elara with project documentation, CI-ready defaults, and empty crates.

### M0.1 Create workspace skeleton

Expected changes:

- Root `Cargo.toml` workspace.
- `crates/` directory.
- Placeholder crates for core, syntax, compiler, bytecode, interpreter, stdlib, API, JIT, C API, test utilities, and benchmarks.
- Root `README.md` with project positioning.
- `docs/` directory containing architecture, milestone, goal, and progress docs.

Verification:

```bash
cargo metadata --format-version 1
cargo fmt --all -- --check
```

Commit:

```text
chore(workspace): bootstrap elara workspace
```

### M0.2 Add lint, formatting, and CI configuration

Expected changes:

- Shared lint configuration in workspace `Cargo.toml`.
- `.rustfmt.toml` if needed.
- GitHub Actions or equivalent CI for fmt, clippy, test.
- Basic `xtask` placeholder if desired.

Verification:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Commit:

```text
chore(ci): add workspace quality gates
```

### M0.3 Define crate-level module policies

Expected changes:

- Each crate has `lib.rs` with module-level docs.
- `elara` top-level crate re-exports only stable public API placeholders.
- `docs/PROGRESS.md` says M0 is in progress or complete.

Verification:

```bash
cargo test --workspace
```

Commit:

```text
docs(workspace): document crate boundaries
```

Exit criteria:

- Workspace builds.
- Empty crates compile.
- CI commands are documented and runnable.
- No source file approaches 1000 lines.

## Milestone M1: Language Specification and Test Harness

Goal: establish Lua 5.5 as the only language target and create test infrastructure before implementing semantics.

### M1.1 Add spec module

Expected changes:

- `elara-core` or `elara-bytecode` exposes `LuaVersion` and `LUA_SPEC`.
- Constants for current language target.
- No `LuaDialect` enum for old versions.

Verification:

```bash
cargo test -p elara-core
```

Commit:

```text
feat(core): define current lua spec
```

### M1.2 Add diagnostic and source span primitives

Expected changes:

- `SourceId`, `Span`, `Spanned<T>`.
- Diagnostic severity and structured messages.
- Basic display tests.

Verification:

```bash
cargo test -p elara-core diagnostics
```

Commit:

```text
feat(core): add source spans and diagnostics
```

### M1.3 Add test fixture layout

Expected changes:

- `tests/fixtures/pass/`.
- `tests/fixtures/fail/`.
- `tests/conformance/`.
- `tests/differential/` placeholder harness docs.

Verification:

```bash
cargo test --workspace
```

Commit:

```text
test(harness): add lua fixture layout
```

### M1.4 Add snapshot testing baseline

Expected changes:

- Snapshot helper for AST/bytecode/diagnostics.
- First trivial fixture: `return 42`.

Verification:

```bash
cargo test --workspace snapshots
```

Commit:

```text
test(harness): add snapshot baseline
```

Exit criteria:

- The repository has a single declared Lua target.
- Tests can store fixtures and snapshots.
- Diagnostics have spans.

## Milestone M2: Runtime Value Model and Basic GC Skeleton

Goal: implement basic `Value`, GC object headers, roots, and allocation scaffolding.

### M2.1 Implement Value and primitive conversions

Expected changes:

- `Value`, `ValueTag`, primitive constructors.
- `Nil`, bool, integer, float support.
- Equality for primitive values.
- Numeric helper functions.

Verification:

```bash
cargo test -p elara-core value
```

Commit:

```text
feat(core): add lua value primitives
```

### M2.2 Add GC pointer and object header types

Expected changes:

- `GcHeader`, `GcKind`, `GcColor`.
- `GcRef<T>` internal pointer wrapper.
- No public raw GC pointer exposure.
- Safety comments for unsafe pointer helpers.

Verification:

```bash
cargo test -p elara-core gc
cargo clippy -p elara-core --all-targets -- -D warnings
```

Commit:

```text
feat(core): add gc object header and references
```

### M2.3 Implement basic arena/list allocator

Expected changes:

- Runtime-owned allocation list.
- Allocation stats.
- Simple root set placeholder.
- Basic drop cleanup for all allocated objects.

Verification:

```bash
cargo test -p elara-core gc_alloc
```

Commit:

```text
feat(core): add gc allocation list
```

### M2.4 Implement stop-the-world mark-sweep MVP

Expected changes:

- Mark phase over roots.
- Sweep phase over allocated object list.
- Root API for tests.
- Tests for reachable/unreachable collection.

Verification:

```bash
cargo test -p elara-core gc_mark_sweep
```

Commit:

```text
feat(core): implement mark sweep gc skeleton
```

Exit criteria:

- Primitive values work.
- GC-managed objects can be allocated and reclaimed under tests.
- Unsafe code is localized and documented.

## Milestone M3: Strings and Tables

Goal: implement the two core data structures required before parser/compiler execution becomes useful.

### M3.1 Implement Lua strings and interning

Expected changes:

- Short string type.
- Long string type.
- String interner in VM state.
- String equality and hashing.

Verification:

```bash
cargo test -p elara-core string
```

Commit:

```text
feat(core): add lua strings and interning
```

### M3.2 Implement table array storage

Expected changes:

- `Table` with array part.
- Raw get/set integer index helpers.
- Nil assignment behavior.
- Array growth tests.

Verification:

```bash
cargo test -p elara-core table_array
```

Commit:

```text
feat(core): add table array storage
```

### M3.3 Implement table hash storage

Expected changes:

- Hash part with Lua `Value` keys.
- Numeric key canonicalization.
- Rehash logic.
- Tests for string, integer, float, boolean keys.

Verification:

```bash
cargo test -p elara-core table_hash
```

Commit:

```text
feat(core): add table hash storage
```

### M3.4 Add metatable field and table versioning

Expected changes:

- Optional metatable pointer.
- Meta flags placeholder.
- Version bump on structural mutations.
- Tests for version changes.

Verification:

```bash
cargo test -p elara-core table_meta
```

Commit:

```text
feat(core): add table metadata versioning
```

Exit criteria:

- Strings and tables are usable by later parser/compiler/runtime steps.
- Table internals do not depend on standard library or JIT.

## Milestone M4: Lexer and Parser

Goal: parse current Lua source into an AST with spans and diagnostics.

### M4.1 Implement lexer for tokens and literals

Expected changes:

- Keywords, identifiers, punctuation, operators.
- Integer/float/string literal tokenization.
- Comments and whitespace handling.
- Error diagnostics for invalid tokens.

Verification:

```bash
cargo test -p elara-syntax lexer
```

Commit:

```text
feat(syntax): add lua lexer
```

### M4.2 Implement expression parser

Expected changes:

- Operator precedence.
- Unary/binary operators.
- Function call expressions.
- Table constructors.
- Vararg expression.

Verification:

```bash
cargo test -p elara-syntax expr
```

Commit:

```text
feat(syntax): parse lua expressions
```

### M4.3 Implement statement parser

Expected changes:

- Assignment.
- Local declarations.
- Global declarations for current Lua.
- Function declarations.
- If/while/repeat/for.
- Return/break/goto/labels where current Lua supports them.

Verification:

```bash
cargo test -p elara-syntax stmt
```

Commit:

```text
feat(syntax): parse lua statements
```

### M4.4 Implement parser snapshots and error tests

Expected changes:

- Snapshot tests for representative chunks.
- Error tests for malformed syntax.
- AST pretty debug output.

Verification:

```bash
cargo test -p elara-syntax
```

Commit:

```text
test(syntax): add parser snapshots
```

Exit criteria:

- Lua chunks parse into AST.
- Syntax errors report spans.
- No runtime execution is mixed into parser code.

## Milestone M5: Bytecode Model and Compiler MVP

Goal: define internal bytecode and compile simple chunks into verified Protos.

### M5.1 Define Proto, Instr, and opcode encoding

Expected changes:

- `Proto`, `Instr`, `Op`.
- Constant pool.
- Upvalue descriptors placeholder.
- Debug info placeholder.

Verification:

```bash
cargo test -p elara-bytecode op
```

Commit:

```text
feat(bytecode): define proto and instructions
```

### M5.2 Add bytecode builder and disassembler

Expected changes:

- Builder API.
- Human-readable disassembly.
- Tests for simple instruction sequences.

Verification:

```bash
cargo test -p elara-bytecode disasm
```

Commit:

```text
feat(bytecode): add builder and disassembler
```

### M5.3 Add bytecode verifier

Expected changes:

- Register bounds checks.
- Constant bounds checks.
- Jump target checks.
- Basic call/return layout checks.

Verification:

```bash
cargo test -p elara-bytecode verifier
```

Commit:

```text
feat(bytecode): add verifier
```

### M5.4 Compile constants and arithmetic expressions

Expected changes:

- AST to bytecode for literals.
- Binary arithmetic lowering.
- Return statement lowering.
- Snapshot tests for disassembly.

Verification:

```bash
cargo test -p elara-compiler simple_expr
```

Commit:

```text
feat(compiler): compile simple expressions
```

Exit criteria:

- `return 1 + 2` can compile to verified bytecode.
- Disassembly snapshots are stable.

## Milestone M6: Interpreter MVP

Goal: execute simple bytecode chunks without full Lua semantics.

### M6.1 Add VM state and thread stack

Expected changes:

- `Vm` or `Runtime` state.
- `LuaThread` stack.
- Basic call frame.
- Stack push/pop helpers.

Verification:

```bash
cargo test -p elara-core thread_stack
```

Commit:

```text
feat(core): add vm state and thread stack
```

### M6.2 Implement interpreter loop for constants and arithmetic

Expected changes:

- `LOAD_*`, `ADD`, `SUB`, `MUL`, `DIV`, `IDIV`, `RETURN`.
- Primitive runtime errors.
- Tests executing simple Protos.

Verification:

```bash
cargo test -p elara-interp arithmetic
```

Commit:

```text
feat(interp): execute primitive arithmetic
```

### M6.3 Connect source compile and eval path

Expected changes:

- Public internal function: source -> Proto -> interpreter.
- Test `return 42` from source.
- Test arithmetic source chunks.

Verification:

```bash
cargo test --workspace eval_simple
```

Commit:

```text
feat(runtime): evaluate simple lua chunks
```

Exit criteria:

- Source strings can be parsed, compiled, verified, and executed for simple expressions.
- Interpreter is still small and readable.

## Milestone M7: Variables, Scopes, Closures, and Calls

Goal: implement enough language semantics for real functions and lexical scope.

### M7.1 Implement local variables and assignment

Expected changes:

- Scope resolution.
- Register allocation for locals.
- Multiple assignment basics.
- Tests for local variable behavior.

Verification:

```bash
cargo test -p elara-compiler locals
cargo test -p elara-interp locals
```

Commit:

```text
feat(compiler): lower local variables
```

### M7.2 Implement function Protos and Lua calls

Expected changes:

- Function definitions.
- Call frames.
- Arguments and return values.
- Tail call placeholder or basic support.

Verification:

```bash
cargo test -p elara-interp calls
```

Commit:

```text
feat(interp): execute lua function calls
```

### M7.3 Implement closures and upvalues

Expected changes:

- Upvalue analysis.
- Open/closed upvalue runtime.
- Closure bytecode.
- Tests for nested functions.

Verification:

```bash
cargo test -p elara-interp closures
```

Commit:

```text
feat(runtime): implement closures and upvalues
```

### M7.4 Implement varargs and named vararg table

Expected changes:

- Vararg function handling.
- `...` lowering.
- Named vararg table support for current Lua.
- Multiple return tests.

Verification:

```bash
cargo test -p elara-interp varargs
```

Commit:

```text
feat(runtime): support varargs
```

Exit criteria:

- Recursive functions and closures work.
- Multiple return and varargs work for common cases.

## Milestone M8: Control Flow and Iteration

Goal: implement Lua control flow and loop semantics.

### M8.1 Implement conditional branches

Expected changes:

- `if`, `elseif`, `else` lowering.
- Truthiness semantics.
- Branch bytecode tests.

Verification:

```bash
cargo test -p elara-interp conditionals
```

Commit:

```text
feat(interp): execute conditional branches
```

### M8.2 Implement while and repeat loops

Expected changes:

- Loop lowering.
- Break handling.
- Tests for loop behavior.

Verification:

```bash
cargo test -p elara-interp loops
```

Commit:

```text
feat(interp): execute while and repeat loops
```

### M8.3 Implement numeric for loops

Expected changes:

- `FOR_PREP`, `FOR_LOOP`.
- Integer and float loop behavior.
- Positive and negative step tests.

Verification:

```bash
cargo test -p elara-interp numeric_for
```

Commit:

```text
feat(interp): execute numeric for loops
```

### M8.4 Implement generic for loops

Expected changes:

- Iterator call protocol.
- `TFOR_*` opcodes.
- Tests using simple iterator functions.

Verification:

```bash
cargo test -p elara-interp generic_for
```

Commit:

```text
feat(interp): execute generic for loops
```

Exit criteria:

- Core control flow works.
- Loops execute through bytecode, not AST interpretation.

## Milestone M9: Tables, Metamethods, and Globals

Goal: implement Lua table operations, global declaration behavior, and metamethod dispatch.

### M9.1 Compile and execute table constructors

Expected changes:

- Array fields.
- Record fields.
- Expression fields.
- Constructor bytecode.

Verification:

```bash
cargo test -p elara-interp table_constructor
```

Commit:

```text
feat(runtime): execute table constructors
```

### M9.2 Implement table get/set bytecode

Expected changes:

- `GET_TABLE`, `SET_TABLE`.
- Fast array path.
- Hash path.
- Nil assignment behavior.

Verification:

```bash
cargo test -p elara-interp table_access
```

Commit:

```text
feat(interp): execute table access
```

### M9.3 Implement metatable and metamethod dispatch

Expected changes:

- `__index`, `__newindex`.
- Arithmetic and comparison metamethods.
- `__len`, `__call`, `__concat`.
- Cold slow path organization.

Verification:

```bash
cargo test -p elara-interp metamethods
```

Commit:

```text
feat(runtime): add metamethod dispatch
```

### M9.4 Implement global declaration semantics

Expected changes:

- Current Lua global declaration validation.
- `_ENV` access model.
- Global get/set bytecode or lowering.
- Tests for declared and undeclared globals.

Verification:

```bash
cargo test -p elara-compiler globals
cargo test -p elara-interp globals
```

Commit:

```text
feat(compiler): enforce global declarations
```

Exit criteria:

- Tables and metamethods work well enough for standard libraries.
- Current Lua global rules are enforced.

## Milestone M10: Errors, Protected Calls, Coroutines, and To-Be-Closed Variables

Goal: complete the hard control-flow features that interact with stack unwinding.

### M10.1 Implement structured runtime errors

Expected changes:

- `LuaError` with traceback support.
- Runtime error creation.
- Error propagation through frames.

Verification:

```bash
cargo test -p elara-interp errors
```

Commit:

```text
feat(runtime): add structured lua errors
```

### M10.2 Implement protected calls

Expected changes:

- Protected call frame markers.
- Error capture.
- Tests for pcall-like behavior.

Verification:

```bash
cargo test -p elara-interp protected_call
```

Commit:

```text
feat(runtime): implement protected calls
```

### M10.3 Implement coroutines and yield/resume

Expected changes:

- Coroutine status transitions.
- Yield from Lua frames.
- Resume result behavior.
- Tests for basic coroutine programs.

Verification:

```bash
cargo test -p elara-interp coroutine
```

Commit:

```text
feat(runtime): implement coroutines
```

### M10.4 Implement to-be-closed variables

Expected changes:

- `TBC` and `CLOSE` lowering.
- Close during normal return.
- Close during errors.
- Close during coroutine interactions.

Verification:

```bash
cargo test -p elara-interp to_be_closed
```

Commit:

```text
feat(runtime): implement to-be-closed variables
```

Exit criteria:

- Stack unwinding is reliable.
- Error handling, coroutine semantics, and close semantics share one coherent mechanism.

## Milestone M11: Standard Library MVP

Goal: implement enough standard library to run meaningful Lua programs.

### M11.1 Add library registration framework

Expected changes:

- `Library` trait or equivalent.
- `StdLibProfile::{Full, Minimal, Sandboxed, Custom}`.
- Registration into globals.

Verification:

```bash
cargo test -p elara-stdlib registry
```

Commit:

```text
feat(stdlib): add library registration profiles
```

### M11.2 Implement base, table, math, and string essentials

Expected changes:

- Base functions needed for tests.
- Table operations.
- Math functions.
- String primitives.

Verification:

```bash
cargo test -p elara-stdlib base table math string
```

Commit:

```text
feat(stdlib): add essential libraries
```

### M11.3 Implement coroutine and utf8 libraries

Expected changes:

- Coroutine standard functions backed by runtime coroutine implementation.
- UTF-8 library basics.

Verification:

```bash
cargo test -p elara-stdlib coroutine utf8
```

Commit:

```text
feat(stdlib): add coroutine and utf8 libraries
```

### M11.4 Add sandboxed profile tests

Expected changes:

- Verify disabled libraries are not registered.
- Verify allowed libraries still work.

Verification:

```bash
cargo test -p elara-stdlib sandbox
```

Commit:

```text
test(stdlib): verify sandbox profile
```

Exit criteria:

- Minimal and sandboxed profiles are usable.
- Full profile has a clear remaining gap list in `PROGRESS.md`.

## Milestone M12: Public Rust Embedding API

Goal: expose a clean Rust API that is safer and easier than the raw Lua C API.

### M12.1 Add `LuaBuilder`, `Lua`, and `Chunk`

Expected changes:

- Build runtime with profile and JIT options.
- Load source chunks.
- Evaluate chunks.
- Basic examples.

Verification:

```bash
cargo test -p elara-api chunk
```

Commit:

```text
feat(api): add lua builder and chunk evaluation
```

### M12.2 Add `IntoLua` and `FromLua`

Expected changes:

- Conversions for primitives.
- String conversion.
- Option conversion.
- Tuple argument conversion basics.

Verification:

```bash
cargo test -p elara-api conversion
```

Commit:

```text
feat(api): add typed lua conversions
```

### M12.3 Add native Rust functions

Expected changes:

- `create_function`.
- Typed argument extraction.
- Multiple return support.
- Error conversion.

Verification:

```bash
cargo test -p elara-api native_function
```

Commit:

```text
feat(api): add native rust functions
```

### M12.4 Add tables, registry keys, and userdata

Expected changes:

- `Table` handle.
- `Function` handle.
- `RegistryKey`.
- Basic `UserData` trait.
- Lifetime-safe handle tests.

Verification:

```bash
cargo test -p elara-api table registry userdata
```

Commit:

```text
feat(api): add tables registry and userdata
```

Exit criteria:

- A Rust user can embed Elara, register functions, pass values, and evaluate Lua code without touching internal VM types.

## Milestone M13: Conformance and Differential Testing

Goal: validate behavior against official Lua for the current version.

### M13.1 Add official Lua runner integration

Expected changes:

- Configurable path to official Lua executable.
- Differential test runner.
- Output and error-class comparison.

Verification:

```bash
cargo test -p elara-test differential_runner
```

Commit:

```text
test(differential): add official lua runner
```

### M13.2 Add conformance test subsets

Expected changes:

- Language fixtures.
- Standard library fixtures.
- Error fixtures.
- Coroutine fixtures.

Verification:

```bash
cargo test --test conformance
```

Commit:

```text
test(conformance): add lua behavior fixtures
```

### M13.3 Add fuzz targets

Expected changes:

- Lexer/parser fuzz target.
- Bytecode verifier fuzz target.
- Table operation fuzz target.

Verification:

```bash
cargo test --workspace
```

Commit:

```text
test(fuzz): add parser and bytecode fuzz targets
```

Exit criteria:

- Conformance gaps are documented in `PROGRESS.md`.
- Differential runner can compare selected scripts with official Lua.

## Milestone M14: Production GC

Goal: evolve GC from correctness MVP to a runtime suitable for real programs.

### M14.1 Implement complete tracing for all object types

Expected changes:

- Trace functions for all runtime objects.
- Stack root tracing.
- Registry root tracing.
- Upvalue tracing.

Verification:

```bash
cargo test -p elara-core gc_trace
```

Commit:

```text
feat(gc): trace all runtime objects
```

### M14.2 Add weak tables and ephemeron behavior

Expected changes:

- Weak key/value modes.
- Ephemeron marking.
- Tests for weak table collection.

Verification:

```bash
cargo test -p elara-core weak_table
```

Commit:

```text
feat(gc): support weak tables
```

### M14.3 Add finalization and userdata lifecycle

Expected changes:

- Finalizer queue.
- Userdata drop behavior.
- Error-safe finalization path.

Verification:

```bash
cargo test -p elara-core finalizer
```

Commit:

```text
feat(gc): add finalization support
```

### M14.4 Add incremental collection and write barriers

Expected changes:

- Tri-color state.
- Barrier calls at mutation sites.
- Tests for incremental invariants.

Verification:

```bash
cargo test -p elara-core incremental_gc
```

Commit:

```text
feat(gc): add incremental collection
```

Exit criteria:

- GC correctness tests cover all object kinds.
- Incremental mode can run without breaking runtime semantics.

## Milestone M15: Interpreter Optimization

Goal: make the interpreter competitive with the reference C Lua implementation on representative workloads.

### M15.1 Add benchmark harness

Expected changes:

- Microbenchmarks for arithmetic, table access, calls, strings.
- Macro benchmarks for typical Lua workloads.
- Optional official Lua comparison scripts.

Verification:

```bash
cargo bench -p elara-bench
```

Commit:

```text
bench(runtime): add interpreter benchmarks
```

### M15.2 Optimize VM dispatch and stack access

Expected changes:

- Verified unchecked stack access in hot paths.
- Fewer temporary allocations.
- Cold slow paths.
- Safety comments.

Verification:

```bash
cargo test -p elara-interp
cargo bench -p elara-bench
```

Commit:

```text
perf(interp): optimize dispatch hot path
```

### M15.3 Add inline caches

Expected changes:

- Table/global access ICs.
- Metatable version guards.
- Invalidation on table version changes.

Verification:

```bash
cargo test -p elara-interp inline_cache
cargo bench -p elara-bench
```

Commit:

```text
perf(interp): add table inline caches
```

### M15.4 Add selected superinstructions

Expected changes:

- Bytecode frequency analysis.
- A small number of high-value superinstructions.
- Correct fallback behavior.

Verification:

```bash
cargo test -p elara-interp superinstruction
cargo bench -p elara-bench
```

Commit:

```text
perf(interp): add selected superinstructions
```

Exit criteria:

- Performance report exists.
- Interpreter is near the reference C Lua implementation on selected representative benchmarks or gaps are clearly identified.

## Milestone M16: Cranelift Baseline JIT

Goal: add optional baseline JIT that is semantically equivalent to the interpreter.

### M16.1 Add JIT crate and feature flag

Expected changes:

- `elara-jit` depends on Cranelift crates.
- `jit` feature in top-level crate.
- `JitMode::{Off, Hot, Always}` API placeholder.

Verification:

```bash
cargo test --workspace --features jit
```

Commit:

```text
feat(jit): add cranelift feature crate
```

### M16.2 Define JIT ABI and runtime helper layer

Expected changes:

- `JitFn` ABI.
- `JitStatus`.
- Runtime helper registration.
- Tests for helper calls without generated Lua code.

Verification:

```bash
cargo test -p elara-jit --features jit abi
```

Commit:

```text
feat(jit): define jit abi and helpers
```

### M16.3 Lower simple arithmetic Protos to Cranelift

Expected changes:

- Cranelift function builder setup.
- Lower constants and arithmetic.
- Execute generated code.
- Compare interpreter and JIT results.

Verification:

```bash
cargo test -p elara-jit --features jit arithmetic
```

Commit:

```text
feat(jit): compile arithmetic protos
```

### M16.4 Add JIT call integration and hot counters

Expected changes:

- Proto hot counters.
- JIT entry pointer.
- Interpreter to JIT transition.
- JIT to interpreter fallback.

Verification:

```bash
cargo test --workspace --features jit jit_transition
```

Commit:

```text
feat(jit): add hot function compilation
```

Exit criteria:

- Simple hot functions can run through Cranelift JIT.
- JIT can be disabled with no semantic difference.

## Milestone M17: JIT Guards, Deoptimization, and Hot Table Paths

Goal: make JIT useful for realistic Lua code while preserving correctness.

### M17.1 Add guard and deopt metadata

Expected changes:

- `DeoptPoint` structures.
- Live register syncing to VM stack.
- Deopt back to interpreter.

Verification:

```bash
cargo test -p elara-jit --features jit deopt
```

Commit:

```text
feat(jit): add guard deoptimization
```

### M17.2 Lower table array fast path

Expected changes:

- Fast array get/set with tag checks.
- Slow helper fallback.
- Table version guard.

Verification:

```bash
cargo test -p elara-jit --features jit table_array
```

Commit:

```text
feat(jit): compile table array fast path
```

### M17.3 Lower calls through trampoline

Expected changes:

- JIT call helper.
- Native call fallback.
- Lua call fallback.
- Yield/error return statuses.

Verification:

```bash
cargo test -p elara-jit --features jit calls
```

Commit:

```text
feat(jit): route calls through trampoline
```

### M17.4 Add JIT equivalence suite

Expected changes:

- Run selected conformance fixtures with interpreter and JIT.
- Compare results.
- Disable JIT automatically for unsupported debug/yield cases.

Verification:

```bash
cargo test --workspace --features jit jit_equivalence
```

Commit:

```text
test(jit): add interpreter equivalence suite
```

Exit criteria:

- JIT can speed up selected hot code.
- Guard failures return to interpreter correctly.
- GC safepoints are conservative and correct.

## Milestone M18: Full Standard Library, Debug Support, and Binary Chunk Policy

Goal: close completeness gaps for the current Lua version.

### M18.1 Complete remaining standard libraries

Expected changes:

- `io`, `os`, `package`, and debug libraries according to selected profiles.
- Host-sensitive functions gated by profile.
- Tests for profile behavior.

Verification:

```bash
cargo test -p elara-stdlib
```

Commit:

```text
feat(stdlib): complete standard library profiles
```

### M18.2 Add debug library frame materialization

Expected changes:

- Debug frame inspection.
- Local/upvalue access where supported.
- JIT deopt or JIT disable behavior for debug hooks.

Verification:

```bash
cargo test -p elara-stdlib debug
cargo test --workspace --features jit debug
```

Commit:

```text
feat(debug): materialize runtime frames
```

### M18.3 Add internal bytecode dump/load

Expected changes:

- Elara internal bytecode serialization.
- Magic/version/header checks.
- Refuse incompatible bytecode.

Verification:

```bash
cargo test -p elara-bytecode dump_load
```

Commit:

```text
feat(bytecode): add internal dump load format
```

### M18.4 Decide and implement official chunk support scope

Expected changes:

- Document whether official Lua chunks are supported.
- If implemented, support only the current Lua version.
- Tests for valid/invalid chunks.

Verification:

```bash
cargo test -p elara-bytecode official_chunk
```

Commit:

```text
feat(bytecode): handle current lua chunks
```

Exit criteria:

- Full profile is close to current Lua behavior.
- Debug interactions with JIT are explicit.
- Bytecode loading has a clear compatibility policy.

## Milestone M19: Optional Current-Version C API

Goal: provide source-level C API compatibility for the current Lua version only.

### M19.1 Add C API crate and headers

Expected changes:

- `elara-capi` crate.
- `lua.h`, `lauxlib.h`, `lualib.h` for current Lua target.
- Build script for header generation or packaging.

Verification:

```bash
cargo test -p elara-capi
```

Commit:

```text
feat(capi): add current lua headers
```

### M19.2 Implement stack-based C API core

Expected changes:

- `lua_State` wrapper.
- Stack push/get/set functions.
- Type inspection.
- Error boundary policy.

Verification:

```bash
cargo test -p elara-capi stack
```

Commit:

```text
feat(capi): implement stack api core
```

### M19.3 Implement C function registration and calls

Expected changes:

- C function closure wrapper.
- Protected call boundary.
- Panic containment.

Verification:

```bash
cargo test -p elara-capi c_function
```

Commit:

```text
feat(capi): support c function calls
```

### M19.4 Add C integration tests

Expected changes:

- Small C module compiled against Elara headers.
- Test loading/registering module.
- Source-level compatibility verification.

Verification:

```bash
cargo test -p elara-capi --tests
```

Commit:

```text
test(capi): add c integration module
```

Exit criteria:

- Current-version C modules can be compiled against Elara headers for core API use.
- Binary compatibility is still not promised unless explicitly added later.

## Milestone M20: Release Hardening and 1.0 Candidate

Goal: stabilize Elara as a usable latest-Lua Rust runtime.

### M20.1 Complete conformance gap review

Expected changes:

- `docs/PROGRESS.md` gap list updated.
- Known unsupported features either implemented or explicitly scoped out.
- Conformance dashboard or summary.

Verification:

```bash
cargo test --workspace
cargo test --workspace --features jit
```

Commit:

```text
docs(progress): summarize release conformance gaps
```

### M20.2 Complete performance report

Expected changes:

- Interpreter vs official Lua benchmarks.
- JIT vs interpreter benchmarks.
- API overhead benchmarks.

Verification:

```bash
cargo bench -p elara-bench
```

Commit:

```text
bench(runtime): add release performance report
```

### M20.3 Audit unsafe and public API

Expected changes:

- Unsafe inventory.
- Safety comments complete.
- Public API docs.
- Examples compile.

Verification:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
```

Commit:

```text
chore(release): audit safety and public api
```

### M20.4 Prepare release candidate

Expected changes:

- README complete.
- Examples complete.
- Version constants correct.
- Tag plan documented.

Verification:

```bash
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Commit:

```text
chore(release): prepare elara release candidate
```

Exit criteria:

- Elara implements the current Lua language target.
- Rust embedding API is stable enough for real use.
- Interpreter is optimized and measured.
- JIT is optional, tested, and semantically equivalent for supported code paths.
- Remaining limitations are explicit and not hidden.
