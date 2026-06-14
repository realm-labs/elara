//! Test harness utilities for Elara.
//!
//! This crate owns test-only utilities for conformance, differential testing,
//! fixtures, and snapshots.
//!
//! It may depend broadly on internal crates for testing, but production crates
//! must not depend on it.

pub mod differential;
pub mod fuzz;
pub mod snapshots;

pub use differential::{
    DifferentialComparison, DifferentialRunner, LuaRunner, OFFICIAL_LUA_ENV, RunClass, RunOutput,
};
pub use fuzz::{fuzz_bytecode_verifier, fuzz_parser_input, fuzz_table_operations};
pub use snapshots::{
    SnapshotKind, assert_snapshot_eq, format_diagnostics_snapshot, normalize_snapshot_text,
    snapshot_path,
};
