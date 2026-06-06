//! Core runtime primitives for Elara.
//!
//! This crate owns the runtime object model: values, GC headers and allocation,
//! strings, tables, closures, threads, errors, and other execution state shared
//! by the interpreter, standard library, public API, and JIT.
//!
//! It must remain independent from syntax parsing, compilation, standard library
//! policy, and JIT lowering details.
