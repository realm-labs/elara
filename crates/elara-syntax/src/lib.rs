//! Lexer, parser, AST, and source diagnostics for current Lua.
//!
//! This crate owns source text handling for the current Lua language target:
//! lexing, parsing, AST construction, and syntax-oriented diagnostics.
//!
//! It must not execute code or depend on runtime, standard library, interpreter,
//! or JIT internals.

pub mod ast;
pub mod expr;
pub mod lexer;
pub mod stmt;
pub mod token;

pub use ast::{
    BinaryOp, Block, Expr, ExprKind, FunctionBody, FunctionName, FunctionScope, GlobalDecl,
    IfClause, NameDecl, Param, Stmt, StmtKind, TableField, TableFieldKind, UnaryOp,
};
pub use expr::{ParsedExpression, parse_expression};
pub use lexer::{Lexed, lex};
pub use stmt::{ParsedChunk, parse_chunk};
pub use token::{Token, TokenKind};

#[cfg(test)]
mod parser_snapshots;
