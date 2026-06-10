//! Bytecode interpreter for Elara.
//!
//! This crate owns the tier-0 bytecode interpreter and its dispatch machinery.
//! It executes verified Elara bytecode against runtime state from `elara-core`.
//!
//! It must not parse Lua source or depend on Cranelift internals.

pub mod primitive;

pub use primitive::{
    CoroutineResume, NativeContext, PrimitiveCoroutine, ProtectedRuntimeOutput, RuntimeEnvironment,
    RuntimeError, RuntimeErrorKind, RuntimeNatives, RuntimeResult, execute_proto,
    execute_proto_protected, execute_proto_with_environment, execute_proto_with_natives,
    execute_proto_with_output,
};
