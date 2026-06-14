//! High-level Lua runtime and chunk evaluation API.

use std::cell::{Cell, RefCell};

use elara_compiler::compile_simple_chunk;
use elara_core::{SourceId, Value};
use elara_interp::execute_proto_with_environment;
use elara_stdlib::StdLibProfile;

use crate::{EvalError, Function, stdlib::runtime_environment_for_stdlib};

/// Builder for a Lua runtime handle.
#[derive(Clone, Debug)]
pub struct LuaBuilder {
    stdlib_profile: StdLibProfile,
}

impl LuaBuilder {
    /// Creates a builder with the full standard-library profile.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects the standard-library profile used by chunks loaded through this runtime.
    #[must_use]
    pub fn stdlib_profile(mut self, profile: StdLibProfile) -> Self {
        self.stdlib_profile = profile;
        self
    }

    /// Builds a Lua runtime handle.
    #[must_use]
    pub fn build(self) -> Lua {
        Lua {
            stdlib_profile: self.stdlib_profile,
            next_source_id: Cell::new(0),
            native_globals: RefCell::new(Vec::new()),
        }
    }
}

impl Default for LuaBuilder {
    fn default() -> Self {
        Self {
            stdlib_profile: StdLibProfile::Full,
        }
    }
}

/// High-level Lua runtime handle for loading and evaluating chunks.
#[derive(Debug)]
pub struct Lua {
    stdlib_profile: StdLibProfile,
    next_source_id: Cell<u32>,
    native_globals: RefCell<Vec<NativeGlobal>>,
}

impl Lua {
    /// Creates a builder for a Lua runtime handle.
    #[must_use]
    pub fn builder() -> LuaBuilder {
        LuaBuilder::new()
    }

    /// Creates a Lua runtime with the default builder configuration.
    #[must_use]
    pub fn new() -> Self {
        LuaBuilder::new().build()
    }

    /// Loads source text into a chunk associated with this runtime.
    #[must_use]
    pub fn load(&self, source: impl Into<Box<str>>) -> Chunk<'_> {
        let source_id = SourceId::new(self.next_source_id.get());
        self.next_source_id.set(source_id.get().saturating_add(1));
        Chunk {
            lua: self,
            source_id,
            source: source.into(),
        }
    }

    /// Evaluates source text directly.
    pub fn eval(&self, source: impl Into<Box<str>>) -> Result<Vec<Value>, EvalError> {
        self.load(source).eval()
    }

    /// Creates a typed native Rust function handle.
    pub fn create_function<Args, Returns, F>(&self, function: F) -> Function
    where
        Args: crate::FromLuaMulti,
        Returns: crate::IntoLuaMulti,
        F: Fn(Args) -> Result<Returns, crate::NativeFunctionError> + Send + Sync + 'static,
    {
        Function::new(function)
    }

    /// Registers a native Rust function as a global for future chunk evaluations.
    pub fn set_global_function(&self, name: impl Into<Box<str>>, function: Function) {
        self.native_globals.borrow_mut().push(NativeGlobal {
            name: name.into(),
            function,
        });
    }
}

impl Default for Lua {
    fn default() -> Self {
        Self::new()
    }
}

/// Loaded Lua source chunk.
#[derive(Debug)]
pub struct Chunk<'lua> {
    lua: &'lua Lua,
    source_id: SourceId,
    source: Box<str>,
}

impl Chunk<'_> {
    /// Source identifier assigned by the owning Lua runtime.
    #[must_use]
    pub const fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Evaluates this chunk through the current simple compiler/interpreter path.
    pub fn eval(&self) -> Result<Vec<Value>, EvalError> {
        let compiled = compile_simple_chunk(self.source_id, &self.source);
        if !compiled.diagnostics.is_empty() {
            return Err(EvalError::Diagnostics(compiled.diagnostics));
        }

        let proto = compiled.proto.expect("compiler succeeded without a proto");
        let mut environment = runtime_environment_for_stdlib(&self.lua.stdlib_profile);
        for native in self.lua.native_globals.borrow().iter() {
            let function = native.function.clone();
            environment.register_native_global(native.name.clone(), move |context, args| {
                function.call(context, args)
            });
        }
        execute_proto_with_environment(&proto, environment)
            .map(|output| output.values)
            .map_err(EvalError::Runtime)
    }
}

#[derive(Clone, Debug)]
struct NativeGlobal {
    name: Box<str>,
    function: Function,
}

#[cfg(test)]
mod tests {
    use elara_core::Value;
    use elara_stdlib::{StdLib, StdLibProfile, StdLibSet};

    use super::{Lua, LuaBuilder};

    #[test]
    fn chunk_evaluates_source_with_default_stdlib_profile() {
        let lua = Lua::new();

        assert_eq!(
            lua.load("return math.abs(-7)").eval(),
            Ok(vec![Value::integer(7)])
        );
    }

    #[test]
    fn chunk_builder_selects_stdlib_profile() {
        let lua = LuaBuilder::new()
            .stdlib_profile(StdLibProfile::Custom([StdLib::Math].into_iter().collect()))
            .build();

        assert_eq!(
            lua.eval("return math.sqrt(49)"),
            Ok(vec![Value::float(7.0)])
        );
    }

    #[test]
    fn chunk_builder_can_disable_stdlib_registration() {
        let lua = LuaBuilder::new()
            .stdlib_profile(StdLibProfile::Custom(StdLibSet::new()))
            .build();

        assert!(lua.eval("return math.abs(-7)").is_err());
    }

    #[test]
    fn chunks_receive_distinct_source_ids() {
        let lua = Lua::new();
        let first = lua.load("return 1");
        let second = lua.load("return 2");

        assert_ne!(first.source_id(), second.source_id());
    }
}
