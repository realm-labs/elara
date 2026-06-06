//! Lexer, parser, AST, and source diagnostics for current Lua.
//!
//! This crate owns source text handling for the current Lua language target:
//! lexing, parsing, AST construction, and syntax-oriented diagnostics.
//!
//! It must not execute code or depend on runtime, standard library, interpreter,
//! or JIT internals.
