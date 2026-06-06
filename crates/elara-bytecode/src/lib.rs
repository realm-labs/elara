//! Internal bytecode definitions and verification for Elara.
//!
//! This crate owns Elara's register bytecode format, prototypes, verifier, and
//! disassembly helpers. The bytecode is the execution contract shared by the
//! interpreter and the optional JIT.
//!
//! It may use core runtime types, but it must not parse Lua source or depend on
//! Cranelift.
