# Elara

Elara is a Rust-native implementation of the current stable Lua language target,
currently Lua 5.5 / Lua 5.5.0. It provides a layered VM pipeline, a high-level
Rust embedding API, an optimized bytecode interpreter, and an optional
Cranelift-backed JIT for supported hot paths.

The main branch tracks one Lua language target at a time. Older Lua behavior is
expected to live in historical tags or maintenance branches, not in runtime
compatibility modes.

## Status

Elara is preparing its first release candidate. The workspace has implemented
the core runtime, parser, bytecode model, simple compiler path, interpreter,
standard-library subsets, public Rust API, optional JIT, optional C API header
surface, conformance smoke tests, differential-test utilities, fuzz entry
points, and release benchmark reporting.

Known release-candidate limitations are explicit:

- A release-sized official Lua conformance corpus is not yet included.
- Differential utilities exist, but the fixture set is still small.
- Bitwise opcode execution and corresponding metamethod dispatch are still a
  product gap.
- Coroutine yielding, file-handle-backed `io`, dynamic Lua/C loading, host
  process termination through `os.exit`, and some string pattern/format corners
  are intentionally scoped as unsupported or partial in this candidate.
- The `elara-capi` crate targets source-level Lua 5.5 header compatibility for
  tested stack/call usage. Binary compatibility with existing Lua modules is
  not promised.

## Quick Start

Add the facade crate from this workspace:

```toml
[dependencies]
elara = { path = "crates/elara" }
```

Evaluate Lua source:

```rust
use elara::Lua;

let lua = Lua::new();
let values = lua.eval("local answer = 40 + 2\nreturn answer").unwrap();
assert_eq!(values.first().and_then(|value| value.as_integer()), Some(42));
```

Register a typed native Rust function:

```rust
use elara::{Lua, NativeFunctionError};

let lua = Lua::new();
let add = lua.create_function(|(left, right): (i64, i64)| {
    Ok::<(i64,), NativeFunctionError>((left + right,))
});
lua.set_global_function("add", add);

let values = lua.eval("return add(20, 22)").unwrap();
assert_eq!(values.first().and_then(|value| value.as_integer()), Some(42));
```

Enable the optional JIT feature:

```toml
[dependencies]
elara = { path = "crates/elara", features = ["jit"] }
```

```rust
use elara::{JitMode, Lua};

let lua = Lua::builder().jit(JitMode::Always).build();
let values = lua.eval("return 20 + 22").unwrap();
assert_eq!(values.first().and_then(|value| value.as_integer()), Some(42));
```

Compile-checked examples live in `crates/elara/examples`:

```bash
cargo run -p elara --example basic_embed
cargo run -p elara --features jit --example jit_embed
```

## Workspace

The repository is organized as a Cargo workspace:

- `elara`: stable public facade for embedders.
- `elara-api`: public Rust embedding API implementation.
- `elara-core`: runtime values, GC, tables, strings, closures, and threads.
- `elara-syntax`: lexer, parser, AST, and source diagnostics.
- `elara-compiler`: semantic analysis and lowering to bytecode.
- `elara-bytecode`: internal opcodes, prototypes, verifier, and disassembler.
- `elara-interp`: bytecode interpreter.
- `elara-stdlib`: standard library implementation.
- `elara-jit`: optional Cranelift JIT.
- `elara-capi`: optional Lua 5.5 C API compatibility layer.
- `elara-test`: conformance, differential, snapshot, and fuzz-target utilities.
- `elara-bench`: benchmark harness support.

## Verification

Release-candidate verification:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features --all-targets
cargo doc --workspace --all-features --no-deps
```

Benchmark report:

```bash
cargo bench -p elara-bench
```

The benchmark harness emits interpreter API, JIT API, and official Lua rows when
`lua5.5` is on `PATH` or `ELARA_LUA` points at a reference interpreter.

## Release Docs

- `docs/ARCHITECTURE.md`: design boundaries and invariants.
- `docs/MILESTONES.md`: implementation plan and release exit criteria.
- `docs/PROGRESS.md`: current state and remaining limitations.
- `docs/PERFORMANCE.md`: release benchmark report.
- `docs/SAFETY_API_AUDIT.md`: unsafe Rust and public API audit.
- `docs/RELEASE.md`: version matrix and tag plan.
