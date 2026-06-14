//! Internal bytecode definitions and verification for Elara.
//!
//! This crate owns Elara's register bytecode format, prototypes, verifier, and
//! disassembly helpers. The bytecode is the execution contract shared by the
//! interpreter and the optional JIT.
//!
//! It may use core runtime types, but it must not parse Lua source or depend on
//! Cranelift.

pub mod builder;
pub mod disasm;
pub mod dump_load;
pub mod op;
pub mod proto;
pub mod verifier;

pub use builder::ProtoBuilder;
pub use disasm::disassemble;
pub use dump_load::{
    DumpError, LoadError, dump_proto, is_current_official_lua_chunk, is_official_lua_chunk,
    load_proto,
};
pub use op::{A_BITS, B_BITS, C_BITS, Instr, MAX_A, MAX_B, MAX_C, OP_BITS, Op};
pub use proto::{
    ConstantIndex, DebugInfo, LocalVarDesc, Proto, Register, StringIndex, UpvalueDesc, UpvalueIndex,
};
pub use verifier::{VerifyError, VerifyErrorKind, verify_proto};
