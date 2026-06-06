//! Lua semantic analysis and lowering for Elara bytecode.
//!
//! This crate owns semantic analysis, HIR construction, and lowering from the
//! current Lua AST into Elara bytecode.
//!
//! It may depend on syntax, bytecode, and core diagnostics or types. It must not
//! execute bytecode, know interpreter internals, or special-case JIT behavior.

pub mod simple_expr;

pub use simple_expr::{CompileResult, compile_simple_chunk};
