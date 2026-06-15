# Safety and Public API Audit

Status: M20.3 release audit  
Last updated: 2026-06-15

This document records the release-candidate audit for unsafe Rust and the
public embedding API surface. It is an inventory of current unsafe boundaries,
not a promise that the implementation is feature-complete.

## Enforcement

- `unsafe_op_in_unsafe_fn` is denied at the workspace level.
- `clippy::undocumented_unsafe_blocks` is denied at the workspace level.
- `clippy::missing_safety_doc` is denied at the workspace level.
- Public facade examples live under crate `examples/` so
  `cargo test --workspace --all-targets` compile-checks them.

## Unsafe Inventory

| Area | Unsafe boundary | Audit result |
|---|---|---|
| `elara-core::gc` | Typed `GcRef` borrows and arena object tracing use raw pointers internally. | Public unsafe methods document allocation, type, movement, and lifetime requirements; unsafe blocks have local `SAFETY` comments. |
| `elara-core::thread` | Checked-once stack slot accessors provide unchecked reads and writes for interpreter hot paths. | Unsafe methods document the bounds precondition; interpreter call sites check bounds before entering unsafe code. |
| `elara-interp::primitive` | Runtime string references and hot register access use GC references and unchecked stack helpers. | Call sites document arena ownership and explicit bounds checks. |
| `elara-jit` | Finalized Cranelift function addresses are transmuted and invoked through the JIT ABI; C-style trampolines dereference runtime context pointers. | Unsafe sites document finalized-code provenance, ABI expectations, and context pointer provenance. |
| `elara-capi` | Exported Lua C API functions accept raw `lua_State`, string, callback, and out pointers. | Every unsafe exported function has a `# Safety` section; internal raw pointer reads/writes retain local `SAFETY` comments. |
| Tests | GC and C API tests construct controlled raw references and invoke C ABI callbacks. | Test unsafe blocks are scoped to explicit test-owned objects or live C API states. |

## Public API Surface

The stable Rust-facing facade is crate `elara`. It re-exports safe handles and
conversion traits from `elara-api`: `Lua`, `LuaBuilder`, `Chunk`, `Function`,
`Table`, `RegistryKey`, `AnyUserData`, typed conversion traits, and error types.

The public API intentionally does not expose unrooted raw GC pointers. Runtime
internals remain behind `elara-core`, `elara-interp`, `elara-bytecode`,
`elara-compiler`, `elara-stdlib`, and optional `elara-jit` crate boundaries.

The optional `elara-capi` crate exposes unsafe FFI entrypoints for source-level
Lua 5.5 C API compatibility. Those entrypoints are not part of the safe Rust
embedding facade, and binary compatibility with existing Lua modules remains
explicitly out of scope.

## Compile-Checked Example

- `crates/elara/examples/basic_embed.rs` demonstrates direct chunk evaluation
  and registration of a typed native Rust function through the facade crate.
