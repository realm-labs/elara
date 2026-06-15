//! Top-level Elara facade crate.
//!
//! This crate is the stable public entry point for embedders. It should expose
//! only the high-level Rust API surface from `elara-api`, not parser,
//! interpreter, bytecode, GC, or JIT internals.
//!
//! The facade is intentionally thin during bootstrap. Public items should be
//! re-exported here only after they are safe embedding abstractions rather than
//! raw runtime implementation details.
//!
//! # Example
//!
//! ```rust
//! use elara::{Lua, NativeFunctionError};
//!
//! # fn main() {
//! let lua = Lua::new();
//! let add = lua.create_function(|(left, right): (i64, i64)| {
//!     Ok::<(i64,), NativeFunctionError>((left + right,))
//! });
//! lua.set_global_function("add", add);
//!
//! let values = lua.eval("return add(20, 22)").expect("chunk should evaluate");
//! assert_eq!(values.first().and_then(|value| value.as_integer()), Some(42));
//! # }
//! ```

pub use elara_api as api;
pub use elara_api::{
    AnyUserData, Chunk, ConversionError, EvalError, FromLua, FromLuaMulti, Function, IntoLua,
    IntoLuaMulti, Lua, LuaBuilder, LuaValue, NativeFunctionError, RegistryError, RegistryKey,
    Table, UserData,
};

#[cfg(feature = "jit")]
pub use elara_api::JitMode;
