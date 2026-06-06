//! Bytecode interpreter for Elara.
//!
//! This crate owns the tier-0 bytecode interpreter and its dispatch machinery.
//! It executes verified Elara bytecode against runtime state from `elara-core`.
//!
//! It must not parse Lua source or depend on Cranelift internals.
