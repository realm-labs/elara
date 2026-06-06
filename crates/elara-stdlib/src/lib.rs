//! Lua standard library support for Elara.
//!
//! This crate owns the standard libraries and profiles exposed to Lua programs.
//! It should build on the runtime and public API layers instead of reaching into
//! parser, compiler, or JIT internals.
//!
//! Standard library behavior targets the current stable Lua version only.
