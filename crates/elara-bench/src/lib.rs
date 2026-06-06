//! Benchmark harness support for Elara.
//!
//! This crate is for benchmark-only helpers and runtime performance harnesses.
//! It must not provide production APIs or become a dependency of runtime crates.
//!
//! Benchmarks should exercise the same public or internal execution paths used
//! by Elara rather than carrying independent VM semantics.
