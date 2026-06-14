//! Top-level Elara facade crate.
//!
//! This crate is the stable public entry point for embedders. It should expose
//! only the high-level Rust API surface from `elara-api`, not parser,
//! interpreter, bytecode, GC, or JIT internals.
//!
//! The facade is intentionally thin during bootstrap. Public items should be
//! re-exported here only after they are safe embedding abstractions rather than
//! raw runtime implementation details.

pub use elara_api as api;
pub use elara_api::{Chunk, EvalError, Lua, LuaBuilder};
