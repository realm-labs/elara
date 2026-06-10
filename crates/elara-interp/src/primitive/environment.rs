//! Runtime environment setup for primitive execution.

use elara_core::Value;

use super::{NativeFunction, RuntimeNatives};

/// Initial global environment and native registry for primitive execution.
#[derive(Clone, Default)]
pub struct RuntimeEnvironment {
    natives: RuntimeNatives,
    globals: Vec<InitialGlobal>,
}

impl RuntimeEnvironment {
    /// Creates an empty runtime environment.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            natives: RuntimeNatives::new(),
            globals: Vec::new(),
        }
    }

    /// Creates a runtime environment from an existing native registry.
    #[must_use]
    pub const fn with_natives(natives: RuntimeNatives) -> Self {
        Self {
            natives,
            globals: Vec::new(),
        }
    }

    /// Registers one initial global value.
    pub fn set_global(&mut self, name: impl Into<Box<str>>, value: Value) {
        self.globals.push(InitialGlobal {
            name: name.into(),
            value,
        });
    }

    /// Registers one native function as a callable global and returns its index.
    pub fn register_native_global(
        &mut self,
        name: impl Into<Box<str>>,
        function: NativeFunction,
    ) -> u32 {
        let index = self.natives.push(function);
        self.set_global(name, Value::native_function_index(index));
        index
    }

    pub(super) fn into_parts(self) -> (RuntimeNatives, Vec<InitialGlobal>) {
        (self.natives, self.globals)
    }
}

#[derive(Clone)]
pub(super) struct InitialGlobal {
    name: Box<str>,
    value: Value,
}

impl InitialGlobal {
    pub(super) fn name(&self) -> &[u8] {
        self.name.as_bytes()
    }

    pub(super) const fn value(&self) -> Value {
        self.value
    }
}
