//! Optional Cranelift JIT support for Elara.
//!
//! This crate owns optional Cranelift-based compilation from verified Elara
//! bytecode into native code. JIT behavior must stay semantically equivalent to
//! the interpreter for every supported path.
//!
//! It consumes bytecode and runtime metadata. It must not parse Lua source.
