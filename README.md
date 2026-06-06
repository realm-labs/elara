# Elara

Elara is a Rust-native implementation of the latest stable Lua VM, currently
targeting Lua 5.5. The project is designed around a clean internal bytecode
pipeline, an optimized interpreter, a high-level Rust embedding API, and an
optional Cranelift JIT.

The main branch tracks the current stable Lua language only. Older Lua behavior
is expected to live in historical tags or maintenance branches, not in runtime
compatibility modes.

## Workspace

The repository is organized as a Cargo workspace:

- `elara-core`: runtime values, GC, tables, strings, closures, and threads.
- `elara`: stable public facade for embedders.
- `elara-syntax`: lexer, parser, AST, and source diagnostics.
- `elara-compiler`: semantic analysis and lowering to bytecode.
- `elara-bytecode`: internal opcodes, prototypes, verifier, and disassembler.
- `elara-interp`: bytecode interpreter.
- `elara-stdlib`: standard library implementation.
- `elara-api`: public Rust embedding API.
- `elara-jit`: optional Cranelift JIT.
- `elara-capi`: optional Lua 5.5 C API compatibility layer.
- `elara-test`: conformance and differential test utilities.
- `elara-bench`: benchmark harness support.

See `docs/ARCHITECTURE.md` and `docs/MILESTONES.md` for the current design and
implementation plan.
