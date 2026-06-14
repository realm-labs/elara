//! Public Rust embedding API for Elara.
//!
//! This crate owns the safe API intended for Rust embedders: runtime handles,
//! typed conversions, native functions, table access, registry support, and
//! userdata abstractions.
//!
//! It may compose the compiler, interpreter, standard library, and optional JIT
//! behind safe handles. It must not expose unrooted raw GC pointers or depend on
//! the C API layer.

pub mod chunk;
pub mod conversion;
pub mod handle;
#[cfg(feature = "jit")]
pub mod jit;
pub mod native_function;
pub mod simple_eval;
pub mod stdlib;

pub use chunk::{Chunk, Lua, LuaBuilder};
pub use conversion::{ConversionError, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, LuaValue};
pub use handle::{AnyUserData, RegistryError, RegistryKey, Table, UserData};
#[cfg(feature = "jit")]
pub use jit::JitMode;
pub use native_function::{Function, NativeFunctionError};
pub use simple_eval::{EvalError, eval_simple_source, eval_simple_source_with_stdlib};
