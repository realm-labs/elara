# Elara Architecture

Status: Draft 1  
Project: Elara  
Primary language: Rust  
Target Lua language: latest stable Lua, currently Lua 5.5 / Lua 5.5.0 as of 2026-06-05  
Primary goal: a complete modern Lua VM in Rust with a high-level Rust embedding API and Cranelift JIT support.

## 1. Product Definition

Elara is a Rust-native implementation of the current stable Lua language. It is not a multi-version Lua compatibility layer. The main branch tracks the current stable Lua release. Older Lua behavior is preserved only through old project tags or maintenance branches when we choose to maintain them.

Elara is designed around four pillars:

1. Correct Lua execution for the current stable Lua version.
2. A high-quality Rust API that makes embedding Lua natural and safe.
3. Interpreter performance that is competitive with the reference C implementation of Lua.
4. Optional Cranelift-based JIT execution for hot Lua code.

The project should feel like a complete runtime, not a thin binding around another Lua implementation.

## 2. External Facts and Source Context

As of 2026-06-05, the Lua download page says the current Lua version is Lua 5.5 and the current release is Lua 5.5.0. The Lua version history page lists Lua 5.5.0 as released on 2025-12-22, with major features including global variable declarations, named vararg tables, more compact arrays, and incremental major garbage collection.

Cranelift is a compiler backend intended to be embedded as a library. The Cranelift JIT module emits code and data into memory where the generated code can be called directly.

Reference links:

- Lua download: https://www.lua.org/download.html
- Lua version history: https://www.lua.org/versions.html
- Lua 5.5 manual: https://www.lua.org/manual/5.5/manual.html
- Lua 5.5 readme: https://www.lua.org/manual/5.5/readme.html
- Cranelift overview: https://cranelift.dev/
- Cranelift JITModule docs: https://docs.rs/cranelift-jit/latest/cranelift_jit/struct.JITModule.html
- Cranelift FunctionBuilder docs: https://docs.wasmtime.dev/api/cranelift/prelude/struct.FunctionBuilder.html

Local official Lua source reference:

- `~/Downloads/lua-lua-a5522f0`

When implementing or reviewing Lua language behavior, treat the Lua 5.5 manual
and the local official Lua source tree as the semantic references. The source is
especially important for edge cases where the manual is brief or where parser,
compiler, VM, table, GC, error, coroutine, or standard-library behavior depends
on implementation details. Use the official source to understand behavior, not
to copy Lua's bytecode format or collapse Elara's Rust-native architecture into a
line-by-line port.

## 3. Non-Goals

Elara should not pursue these in the mainline architecture:

- Supporting Lua 5.1, 5.2, 5.3, or 5.4 through runtime compatibility flags.
- Maintaining multiple official Lua bytecode formats in the main execution pipeline.
- Loading arbitrary old Lua binary chunks.
- Becoming a drop-in binary replacement for existing Lua shared libraries.
- Making the first JIT version as aggressive as LuaJIT.
- Sacrificing architectural clarity to chase local benchmark wins.

Older Lua versions may be supported by historical tags, not by accumulating compatibility branches inside the current VM.

## 4. High-Level Pipeline

```text
Lua 5.5 source
    │
    ▼
elara-syntax
lexer + parser
    │
    ▼
AST
    │
    ▼
elara-compiler
semantic analysis + lowering
    │
    ▼
HIR
    │
    ▼
elara-bytecode
internal register bytecode + verifier
    │
    ├───────────────┐
    ▼               ▼
elara-interp     elara-jit
Tier 0 VM        Cranelift tiers
    │               │
    └───────┬───────┘
            ▼
elara-core runtime state
Value / GC / Table / String / Closure / Thread
```

The internal bytecode is the stable execution interface for Elara. The interpreter and JIT both consume it. The JIT must not parse Lua syntax or depend on source-level special cases.

## 5. Workspace Layout

The repository should use a Cargo workspace.

```text
elara/
  Cargo.toml
  README.md
  docs/
    ARCHITECTURE.md
    MILESTONES.md
    CODEX_GOAL.md
    PROGRESS.md
  crates/
    elara/
    elara-core/
    elara-syntax/
    elara-compiler/
    elara-bytecode/
    elara-interp/
    elara-stdlib/
    elara-api/
    elara-jit/
    elara-capi/
    elara-test/
    elara-bench/
  tests/
    conformance/
    differential/
    fixtures/
  benches/
  xtask/
```

### Crate Responsibilities

| Crate | Responsibility | May Depend On | Must Not Depend On |
|---|---|---|---|
| `elara` | Stable public facade crate for Rust embedders | api | core internals, syntax, compiler, interpreter, bytecode, stdlib, JIT, C API internals |
| `elara-core` | Runtime objects, GC, Value, Table, String, Closure, Thread, errors | small utility crates | syntax, compiler, JIT |
| `elara-syntax` | Lexer, parser, AST for current Lua | core diagnostics only | runtime execution, JIT |
| `elara-compiler` | AST to HIR to bytecode | syntax, bytecode, core types | interpreter internals, JIT internals |
| `elara-bytecode` | Internal opcodes, Proto, verifier, disassembler, internal dump/load | core | syntax parser, Cranelift |
| `elara-interp` | Optimized bytecode interpreter | core, bytecode | syntax parser, Cranelift internals |
| `elara-stdlib` | Lua standard libraries and profiles | core, api | parser internals, JIT internals |
| `elara-api` | Public Rust embedding API | core, compiler, interp, stdlib, optional jit | capi internals |
| `elara-jit` | Cranelift JIT, lowering, guards, deopt | core, bytecode, interp trampolines | syntax parser |
| `elara-capi` | Optional Lua 5.5 C API source-compatible layer | core, api | old Lua C APIs |
| `elara-test` | Test harness utilities and differential runner | all internal crates | production dependencies from test code |
| `elara-bench` | Benchmark harness | runtime crates | production dependency from benchmark code |

`elara-core` must remain the center of gravity. Other crates may call into it, but it should not be polluted by parser, standard library, or JIT details.

## 6. Runtime Core

### 6.1 Value Representation

The default value representation should start as a clear tagged value. A later feature can add NaN boxing if benchmarks justify it.

```rust
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Value {
    tag: ValueTag,
    payload: ValuePayload,
}

#[repr(u8)]
pub enum ValueTag {
    Nil,
    Bool,
    Integer,
    Float,
    ShortString,
    LongString,
    Table,
    Closure,
    Thread,
    UserData,
    LightUserData,
}
```

Required properties:

- `Value` must be cheap to copy.
- Hot paths must avoid heap allocation.
- Integer and float behavior follows the current Lua specification.
- Table keys must canonicalize numeric keys according to Lua semantics.
- NaN keys must be rejected consistently with Lua behavior.

### 6.2 Strings

Strings are immutable GC objects.

```text
ShortString:
  interned, suitable for table keys and identifiers.

LongString:
  not necessarily interned.

ExternalString:
  optional support for externally-owned string storage, matching current Lua direction.
```

Short strings should be interned by the runtime state. Equality should first try pointer equality for interned strings.

### 6.3 Tables

Tables are central to Lua performance. Elara should implement split array/hash storage.

```rust
pub struct Table {
    array: CompactArray,
    hash: HashPart,
    metatable: Option<GcRef<Table>>,
    flags: MetaFlags,
    version: u32,
}
```

Design requirements:

- Separate fast array access from hash access.
- Cache missing metamethods with flags.
- Maintain a `version` counter for JIT inline cache invalidation.
- Support weak keys, weak values, and ephemeron behavior.
- Support finalizers where the current Lua version requires them.
- Preserve exact Lua behavior for nil assignment, iteration, length, numeric keys, and metamethod fallback.

### 6.4 Closures and Upvalues

```rust
pub enum Closure {
    Lua(LuaClosure),
    Native(NativeClosure),
    C(CClosure),
}

pub struct LuaClosure {
    proto: GcRef<Proto>,
    upvalues: Box<[UpvalueRef]>,
    jit_entry: AtomicJitEntry,
}

pub enum UpvalueState {
    Open(StackSlot),
    Closed(Value),
}
```

Open upvalues point into a Lua thread stack. Closed upvalues own a `Value` after the stack slot goes out of scope.

### 6.5 Threads, Stacks, and Calls

A Lua thread owns its own stack and call frames.

```rust
pub struct LuaThread {
    stack: ValueStack,
    frames: FrameStack,
    status: ThreadStatus,
    open_upvalues: OpenUpvalueList,
}

pub struct CallFrame {
    closure: GcRef<Closure>,
    pc: u32,
    base: StackIndex,
    top: StackIndex,
    wanted_results: ResultCount,
    flags: FrameFlags,
}
```

The stack model must support:

- Lua calls.
- Rust native calls.
- C API calls.
- Varargs and named vararg table lowering.
- Multiple return values.
- Tail calls.
- Protected calls.
- Coroutine yield/resume.
- To-be-closed variables.
- Debug frame materialization.
- JIT deoptimization.

## 7. Garbage Collection

Elara should implement its own tracing GC. `Rc`, `Arc`, and `RefCell` are not a suitable foundation for the VM object graph.

Recommended phases:

```text
Phase A: stop-the-world mark-sweep GC for correctness.
Phase B: incremental tri-color GC with write barriers.
Phase C: weak table, ephemeron, and finalizer correctness.
Phase D: generational or current-Lua-compatible advanced GC mode.
Phase E: JIT safepoints and stack materialization.
```

Object headers should be explicit.

```rust
#[repr(C)]
pub struct GcHeader {
    kind: GcKind,
    color: GcColor,
    age: GcAge,
    flags: u8,
    next: Option<NonNull<GcHeader>>,
}
```

All object graph mutations must call write barriers when incremental or generational GC is active.

Required barrier sites:

- Table value writes.
- Metatable updates.
- Upvalue writes.
- Userdata user value writes.
- Closure creation.
- Thread stack root changes at safepoints.
- Any native API operation that stores a `Value` into a GC object.

## 8. Internal Bytecode

Elara should use a custom internal register bytecode. It should not copy an official Lua bytecode format as the VM execution contract.

```rust
pub struct Proto {
    pub code: Box<[Instr]>,
    pub constants: Box<[Value]>,
    pub upvalues: Box<[UpvalueDesc]>,
    pub max_stack: u16,
    pub params: u8,
    pub is_vararg: bool,
    pub debug: DebugInfo,
}
```

Example opcode groups:

```text
Load and move:
  MOVE, LOAD_NIL, LOAD_BOOL, LOAD_INT, LOAD_FLOAT, LOAD_K

Upvalues and globals:
  GET_UPVALUE, SET_UPVALUE, GET_ENV, SET_ENV, DECL_GLOBAL

Tables:
  NEW_TABLE, GET_TABLE, SET_TABLE, GET_INDEX, SET_INDEX, LEN

Arithmetic:
  ADD, SUB, MUL, DIV, IDIV, MOD, POW, UNM
  BAND, BOR, BXOR, SHL, SHR, BNOT

Control flow:
  JMP, TEST, TEST_SET, EQ, LT, LE
  FOR_PREP, FOR_LOOP, TFOR_PREP, TFOR_CALL, TFOR_LOOP

Calls:
  CALL, TAIL_CALL, RETURN, VARARG, VARARG_TABLE

Closures and lifetime:
  CLOSURE, CLOSE, TBC
```

The bytecode verifier is mandatory. It must prove enough safety properties that the interpreter can use unchecked indexing in hot paths after verification.

Verifier responsibilities:

- Register indexes are in range.
- Constant indexes are in range.
- Upvalue indexes are in range.
- Jumps target valid instruction boundaries.
- Stack frame size is sufficient.
- `RETURN`, `CALL`, and vararg layouts are valid.
- To-be-closed variable ranges are well-formed.

## 9. Compiler Architecture

```text
source
  -> tokens
  -> AST
  -> semantic scopes
  -> HIR
  -> bytecode builder
  -> bytecode verifier
  -> Proto
```

The compiler must separate parsing from semantic lowering.

Key passes:

1. Lexical tokenization.
2. AST parsing with source spans.
3. Name resolution and scope construction.
4. Global declaration validation.
5. Constant folding for safe constant expressions.
6. Lowering to HIR.
7. Register allocation for Lua registers.
8. Bytecode emission.
9. Debug info emission.
10. Verification.

Diagnostics must preserve spans and produce actionable messages. The compiler should be usable by tools, not only by the VM.

## 10. Interpreter

The interpreter is Tier 0 and must always support all language features.

Design rules:

- Cache `pc`, `base`, and `top` in local variables inside the VM loop.
- Use a bytecode verifier so hot register access can use audited unsafe code.
- Keep slow paths out of the hot dispatch loop.
- Metamethod calls, GC, debug hooks, and errors should be cold paths.
- Add inline caches only after baseline correctness is stable.
- Add superinstructions only after measurement.

The first interpreter should prioritize correctness. The optimized interpreter should target performance close to the reference C Lua implementation.

## 11. Cranelift JIT

The JIT is optional and lives in `elara-jit`. It lowers internal bytecode to Cranelift IR.

### 11.1 Tiering Model

```text
Tier 0: interpreter
  Every function starts here.

Tier 1: baseline method JIT
  Hot Protos are compiled as whole functions.
  Conservative guards and slow runtime helpers are used.

Tier 2: specialized JIT
  Type feedback, inline caches, table version guards, and selected deoptimization.
```

Do not start with a trace JIT. Start with method JIT because it aligns with Proto boundaries and Cranelift function compilation.

### 11.2 JIT ABI

Use a stable C ABI boundary between generated code and Rust runtime helpers.

```rust
pub type JitFn = unsafe extern "C" fn(
    vm: *mut Vm,
    thread: *mut LuaThread,
    frame: *mut CallFrame,
) -> JitStatus;
```

The JIT should read/write the Lua stack rather than passing Lua arguments through native ABI parameters. This keeps varargs, multiple return values, GC roots, and deoptimization manageable.

### 11.3 Runtime Helpers

JIT-generated code should call small runtime helpers for complex operations.

```rust
extern "C" fn rt_add(vm: *mut Vm, a: Value, b: Value, out: *mut Value) -> RuntimeStatus;
extern "C" fn rt_get_table(vm: *mut Vm, table: Value, key: Value, out: *mut Value) -> RuntimeStatus;
extern "C" fn rt_set_table(vm: *mut Vm, table: Value, key: Value, value: Value) -> RuntimeStatus;
extern "C" fn rt_call(vm: *mut Vm, thread: *mut LuaThread, frame: *mut CallFrame) -> RuntimeStatus;
```

### 11.4 Safepoints and Deoptimization

First implementation rule: before any helper that can allocate, call Lua, call a metamethod, yield, or raise an error, the JIT must sync all live Lua values back to the VM stack.

Deopt data:

```rust
pub struct DeoptPoint {
    pub pc: u32,
    pub base: StackIndex,
    pub live_regs: RegBitmap,
}
```

On guard failure:

1. Sync live values to VM stack.
2. Set `frame.pc` to the deopt PC.
3. Return `JitStatus::Deopt`.
4. Resume in the interpreter.

## 12. Rust Embedding API

The public API should be in `elara-api` and re-exported by the top-level `elara` crate.

Example target API:

```rust
let lua = LuaBuilder::new()
    .stdlib(StdLibProfile::Full)
    .jit(JitMode::Hot { threshold: 1_000 })
    .build()?;

lua.globals().set("add", lua.create_function(|_, (a, b): (i64, i64)| {
    Ok(a + b)
})?)?;

let result: i64 = lua.load("return add(20, 22)")
    .set_name("demo.lua")
    .eval()?;
```

Core API traits:

```rust
pub trait IntoLua<'lua> {
    fn into_lua(self, cx: Context<'lua>) -> Result<Value<'lua>>;
}

pub trait FromLua<'lua>: Sized {
    fn from_lua(value: Value<'lua>, cx: Context<'lua>) -> Result<Self>;
}

pub trait UserData {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M);
}
```

API requirements:

- Safe handles with lifetimes tied to the Lua context.
- Registry keys for values that outlive a stack frame.
- Native Rust functions with typed argument conversion.
- Userdata with methods and metamethods.
- Optional Serde integration.
- Optional async integration only after coroutine semantics are stable.
- Sandboxed standard library profiles.
- No public API that exposes raw GC pointers.

## 13. Standard Library Profiles

Do not split standard library behavior by old Lua versions. Split by embedding use case.

```rust
pub enum StdLibProfile {
    Full,
    Minimal,
    Sandboxed,
    Custom(StdLibSet),
}
```

Recommended meaning:

```text
Full:
  Complete current Lua standard library.

Minimal:
  base, table, string, math, coroutine essentials.

Sandboxed:
  excludes dangerous or host-sensitive capabilities such as unrestricted io, os, package loading, and debug.

Custom:
  caller explicitly selects libraries.
```

The standard library should register itself through public or semi-public runtime APIs. It should not mutate internal VM structures directly unless there is a carefully documented reason.

## 14. Optional C API

`elara-capi` is optional and targets the current Lua C API only.

Rules:

- Source-level compatibility is the goal.
- Binary compatibility with existing Lua modules is not a mainline guarantee.
- C modules should be compiled against Elara's headers.
- Rust panics must not cross FFI boundaries.
- C longjmp-like behavior must be contained in a clear trampoline layer.

## 15. Testing Strategy

Testing is a first-class architecture component.

Required layers:

```text
Unit tests:
  Value, Table, String, GC, bytecode verifier, parser pieces.

Snapshot tests:
  AST, HIR, bytecode disassembly, diagnostics.

Conformance tests:
  Lua language behavior and standard library behavior.

Differential tests:
  Run selected scripts on official Lua and Elara; compare outputs and error classes.

Fuzz tests:
  lexer/parser, bytecode verifier, loader, table operations.

JIT equivalence tests:
  Compare interpreter results and JIT results for the same Proto.

Benchmark tests:
  PUC Lua comparison, interpreter microbenchmarks, API overhead, table workloads.
```

A feature is not complete until it has verification coverage.

## 16. Performance Strategy

Do not optimize before semantics are stable. When optimizing, measure every claim.

Priority order:

1. Compact `Value` representation.
2. Table array fast path.
3. Short string interning.
4. Efficient stack and call frame layout.
5. Bytecode verifier enabling unchecked hot reads.
6. Inline caches for table/global/metamethod operations.
7. Superinstructions based on real bytecode frequency.
8. GC barrier tuning.
9. Cranelift baseline JIT.
10. Specialized JIT guards and deoptimization.

Performance target:

- Interpreter should aim for performance competitive with reference C Lua on representative workloads.
- JIT should improve hot numeric loops and stable table access workloads.
- Performance work must not hide correctness bugs or create unmaintainable shortcuts.

## 17. Unsafe Rust Policy

Unsafe code is allowed only in narrow runtime internals where it is justified by VM performance or GC implementation needs.

Rules:

- Every unsafe block must have a `// SAFETY:` explanation.
- Unsafe helpers should be small and tested through safe wrappers.
- Public APIs should be safe unless FFI explicitly requires unsafe.
- GC pointer access must be centralized.
- JIT-generated code must interact through audited ABI helpers.
- No unsafe code should be introduced for convenience.

## 18. Repository Hygiene

Hard constraints:

- Keep the architecture layered.
- Use conventional commit messages.
- Commit by verifiable steps, not by whole milestone dumps.
- Do not let a single source file exceed 1000 lines.
- Prefer modules below 600 lines when practical.
- Keep tests close to the module they validate.
- Update `docs/PROGRESS.md` after each completed step.
- Do not use `PROGRESS.md` as a changelog.
- Do not mix unrelated refactors with feature work.
- Do not add old Lua compatibility branches unless the architecture document is explicitly revised.

## 19. Release and Tag Policy

Main branch tracks the latest stable Lua version.

Suggested branch/tag policy:

```text
main:
  current stable Lua version.

branch/lua-5.5:
  optional maintenance branch after Lua 5.6 becomes the main target.

tag/lua-5.5-complete:
  a known complete support point for Lua 5.5.

tag/vX.Y.Z:
  Elara crate/runtime release tag.
```

Elara's crate version and Lua's language version are separate. The public API follows Rust semver; language conformance follows the current Lua version documented by the project.

## 20. Architectural Invariants

These invariants must remain true throughout the project:

1. The interpreter is the source of semantic completeness.
2. JIT must be optional and semantically equivalent to the interpreter.
3. Bytecode is verified before optimized execution.
4. All GC-managed references are traceable at safepoints.
5. Rust public APIs do not expose unrooted raw GC references.
6. Standard libraries are registered through structured APIs.
7. Old Lua versions are not hidden behind scattered runtime flags.
8. Debug and coroutine behavior can force JIT deoptimization or interpretation.
9. Every milestone ends with executable verification.
10. Clean module boundaries are more important than short-term implementation speed.
