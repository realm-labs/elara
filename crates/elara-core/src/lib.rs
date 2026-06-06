//! Core runtime primitives for Elara.
//!
//! This crate owns the runtime object model: values, GC headers and allocation,
//! strings, tables, closures, threads, errors, and other execution state shared
//! by the interpreter, standard library, public API, and JIT.
//!
//! It must remain independent from syntax parsing, compilation, standard library
//! policy, and JIT lowering details.

pub mod diagnostics;
pub mod gc;
pub mod spec;
pub mod string;
pub mod table;
pub mod value;

pub use diagnostics::{Diagnostic, DiagnosticLabel, DiagnosticSeverity, SourceId, Span, Spanned};
pub use gc::{
    GcArena, GcCollectionStats, GcColor, GcHeader, GcKind, GcObject, GcRef, GcRoot, GcStats,
};
pub use spec::{LUA_SPEC, LUA_VERSION, LuaSpec, LuaVersion};
pub use string::{
    LongString, SHORT_STRING_MAX_BYTES, ShortString, StringInterner, hash_string_bytes,
};
pub use table::Table;
pub use value::{LuaFloat, LuaInteger, Value, ValueTag, float_to_integer_exact};
