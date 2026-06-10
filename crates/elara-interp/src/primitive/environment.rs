//! Runtime environment setup for primitive execution.

use elara_core::Value;

use super::RuntimeNatives;

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
            value: InitialValue::Value(value),
        });
    }

    /// Registers one initial global table with prebuilt field values.
    pub fn set_global_table<I, N>(&mut self, name: impl Into<Box<str>>, fields: I)
    where
        I: IntoIterator<Item = (N, Value)>,
        N: Into<Box<str>>,
    {
        self.globals.push(InitialGlobal {
            name: name.into(),
            value: InitialValue::Table(
                fields
                    .into_iter()
                    .map(|(name, value)| InitialField {
                        name: name.into(),
                        value,
                    })
                    .collect(),
            ),
        });
    }

    /// Registers one native function and returns its runtime index.
    pub fn push_native<F>(&mut self, function: F) -> u32
    where
        F: for<'a> Fn(&mut super::NativeContext<'a>, &[Value]) -> super::RuntimeResult<Vec<Value>>
            + Send
            + Sync
            + 'static,
    {
        self.natives.push(function)
    }

    /// Registers one arg-only native function and returns its runtime index.
    pub fn push_simple_native<F>(&mut self, function: F) -> u32
    where
        F: Fn(&[Value]) -> super::RuntimeResult<Vec<Value>> + Send + Sync + 'static,
    {
        self.natives.push_simple(function)
    }

    /// Registers one native function as a callable global and returns its index.
    pub fn register_native_global<F>(&mut self, name: impl Into<Box<str>>, function: F) -> u32
    where
        F: for<'a> Fn(&mut super::NativeContext<'a>, &[Value]) -> super::RuntimeResult<Vec<Value>>
            + Send
            + Sync
            + 'static,
    {
        let index = self.push_native(function);
        self.set_global(name, Value::native_function_index(index));
        index
    }

    /// Registers one arg-only native function as a callable global.
    pub fn register_simple_native_global<F>(
        &mut self,
        name: impl Into<Box<str>>,
        function: F,
    ) -> u32
    where
        F: Fn(&[Value]) -> super::RuntimeResult<Vec<Value>> + Send + Sync + 'static,
    {
        let index = self.push_simple_native(function);
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
    value: InitialValue,
}

impl InitialGlobal {
    pub(super) fn name(&self) -> &[u8] {
        self.name.as_bytes()
    }

    pub(super) const fn value(&self) -> &InitialValue {
        &self.value
    }
}

#[derive(Clone)]
pub(super) enum InitialValue {
    Value(Value),
    Table(Vec<InitialField>),
}

#[derive(Clone)]
pub(super) struct InitialField {
    name: Box<str>,
    value: Value,
}

impl InitialField {
    pub(super) fn name(&self) -> &[u8] {
        self.name.as_bytes()
    }

    pub(super) const fn value(&self) -> Value {
        self.value
    }
}
