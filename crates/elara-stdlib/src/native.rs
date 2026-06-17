//! Executable native standard-library function descriptors.

use elara_core::{ThreadStatus, Value};

use crate::{FunctionSpec, StdLib};

/// Result returned by executable standard-library natives.
pub type NativeResult = Result<Vec<Value>, NativeError>;

/// Target accepted by `debug.getinfo`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DebugInfoTarget {
    /// Inspect a stack frame by one-based Lua level.
    Level(i64),
    /// Inspect a Lua or native function value.
    Function(Value),
}

/// Stored debug hook metadata for the current runtime thread.
#[derive(Clone, Debug, PartialEq)]
pub struct DebugHookState {
    /// Hook callback function.
    pub function: Value,
    /// Normalized hook event mask bytes in Lua `crl` order.
    pub mask: Vec<u8>,
    /// Instruction-count hook interval.
    pub count: i64,
}

/// Runtime services available to standard-library native functions.
pub trait NativeRuntime {
    /// Interns a short Lua string in the current runtime.
    fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError>;

    /// Allocates a Lua string in the current runtime.
    fn intern_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
        self.intern_short_string(bytes)
    }

    /// Returns bytes for a short Lua string owned by this runtime.
    fn short_string_bytes(&self, value: Value) -> Option<&[u8]>;

    /// Returns bytes for a Lua string owned by this runtime.
    fn string_bytes(&self, value: Value) -> Option<&[u8]> {
        self.short_string_bytes(value)
    }

    /// Allocates a runtime-owned Lua table from raw key/value entries.
    fn create_table(&mut self, _entries: &[(Value, Value)]) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table allocation".into(),
        }
        .into())
    }

    /// Returns the current raw array length of a runtime-owned table.
    fn table_array_len(&self, _table: Value) -> Result<i64, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table length".into(),
        }
        .into())
    }

    /// Reads one raw integer-keyed value from a runtime-owned table.
    fn table_get_integer(&self, _table: Value, _index: i64) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table reads".into(),
        }
        .into())
    }

    /// Reads one raw value-keyed value from a runtime-owned table.
    fn table_get(&self, _table: Value, _key: Value) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table reads".into(),
        }
        .into())
    }

    /// Returns the next raw key/value pair after a key in a runtime-owned table.
    fn table_next(
        &self,
        _table: Value,
        _key: Value,
    ) -> Result<Option<(Value, Value)>, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table iteration".into(),
        }
        .into())
    }

    /// Writes one raw integer-keyed value into a runtime-owned table.
    fn table_set_integer(
        &mut self,
        _table: Value,
        _index: i64,
        _value: Value,
    ) -> Result<(), NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table writes".into(),
        }
        .into())
    }

    /// Writes one raw value-keyed value into a runtime-owned table.
    fn table_set(&mut self, _table: Value, _key: Value, _value: Value) -> Result<(), NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table writes".into(),
        }
        .into())
    }

    /// Reads one raw value from the global environment by name.
    fn global_get(&mut self, _name: &[u8]) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support global reads".into(),
        }
        .into())
    }

    /// Returns the runtime-owned debug registry table.
    fn debug_registry(&mut self) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support debug registry access".into(),
        }
        .into())
    }

    /// Returns the currently installed debug hook metadata, if any.
    fn debug_gethook(&mut self) -> Result<Option<DebugHookState>, NativeError> {
        Ok(None)
    }

    /// Installs debug hook metadata for the current runtime thread.
    fn debug_sethook(&mut self, _hook: DebugHookState) -> Result<(), NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support debug hook installation".into(),
        }
        .into())
    }

    /// Clears debug hook metadata for the current runtime thread.
    fn debug_clearhook(&mut self) -> Result<(), NativeError> {
        Ok(())
    }

    /// Returns a `debug.getinfo` result table, or nil when the target is unavailable.
    fn debug_getinfo(
        &mut self,
        _target: DebugInfoTarget,
        _options: Option<&[u8]>,
    ) -> Result<Value, NativeError> {
        Ok(Value::nil())
    }

    /// Returns a `debug.getlocal` name/value pair, or nil when absent.
    fn debug_getlocal(
        &mut self,
        _level: i64,
        _local: i64,
    ) -> Result<Option<(Value, Value)>, NativeError> {
        Ok(None)
    }

    /// Returns a `debug.getlocal` parameter name for a function target, or nil when absent.
    fn debug_getlocal_function(
        &mut self,
        _function: Value,
        _local: i64,
    ) -> Result<Option<Value>, NativeError> {
        Ok(None)
    }

    /// Sets a `debug.setlocal` target and returns the local name, or nil when absent.
    fn debug_setlocal(
        &mut self,
        _level: i64,
        _local: i64,
        _value: Value,
    ) -> Result<Option<Value>, NativeError> {
        Ok(None)
    }

    /// Returns a `debug.getupvalue` name/value pair, or nil when absent.
    fn debug_getupvalue(
        &mut self,
        _function: Value,
        _index: i64,
    ) -> Result<Option<(Value, Value)>, NativeError> {
        Ok(None)
    }

    /// Sets a `debug.setupvalue` target and returns the upvalue name, or nil when absent.
    fn debug_setupvalue(
        &mut self,
        _function: Value,
        _index: i64,
        _value: Value,
    ) -> Result<Option<Value>, NativeError> {
        Ok(None)
    }

    /// Returns a `debug.upvalueid` light userdata identity, or nil when absent.
    fn debug_upvalueid(
        &mut self,
        _function: Value,
        _index: i64,
    ) -> Result<Option<Value>, NativeError> {
        Ok(None)
    }

    /// Joins one Lua function upvalue to another Lua function upvalue.
    fn debug_upvaluejoin(
        &mut self,
        _target_function: Value,
        _target_index: i64,
        _source_function: Value,
        _source_index: i64,
    ) -> Result<bool, NativeError> {
        Ok(false)
    }

    /// Returns a traceback string for the current debug frame state.
    fn debug_traceback(
        &mut self,
        message: Option<&[u8]>,
        _level: i64,
    ) -> Result<Value, NativeError> {
        let mut output = Vec::new();
        if let Some(message) = message {
            output.extend_from_slice(message);
            output.push(b'\n');
        }
        output.extend_from_slice(b"stack traceback:");
        self.intern_string(&output)
    }

    /// Returns a runtime-owned table's metatable, or nil when absent.
    fn table_metatable(&self, _table: Value) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table metatables".into(),
        }
        .into())
    }

    /// Sets a runtime-owned table's metatable to nil or another table.
    fn table_set_metatable(&mut self, _table: Value, _metatable: Value) -> Result<(), NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support table metatables".into(),
        }
        .into())
    }

    /// Returns the next 64 bits from this runtime's standard-library RNG.
    fn next_random_u64(&mut self) -> Result<u64, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support random numbers".into(),
        }
        .into())
    }

    /// Returns a runtime-provided seed for randomizing the standard-library RNG.
    fn random_seed(&mut self) -> Result<u64, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support random seeding".into(),
        }
        .into())
    }

    /// Replaces this runtime's standard-library RNG state.
    fn set_random_seed(&mut self, _seed1: u64, _seed2: u64) -> Result<(), NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support random seeding".into(),
        }
        .into())
    }

    /// Writes bytes to the host output stream used by base `print`.
    fn write_output(&mut self, _bytes: &[u8]) -> Result<(), NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support output".into(),
        }
        .into())
    }

    /// Returns whether host warnings are currently enabled.
    fn warnings_enabled(&self) -> bool {
        false
    }

    /// Enables or disables host warning emission.
    fn set_warnings_enabled(&mut self, _enabled: bool) -> Result<(), NativeError> {
        Ok(())
    }

    /// Writes bytes to the host warning stream used by base `warn`.
    fn write_warning(&mut self, _bytes: &[u8]) -> Result<(), NativeError> {
        Ok(())
    }

    /// Calls a Lua or native value behind a protected-call boundary.
    fn protected_call(
        &mut self,
        _function: Value,
        _args: &[Value],
    ) -> Result<Result<Vec<Value>, Box<str>>, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support protected calls".into(),
        }
        .into())
    }

    /// Returns a runtime-owned function value for an already registered native helper.
    fn native_function(&self, _library: StdLib, _name: &str) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support native function values".into(),
        }
        .into())
    }

    /// Creates a runtime-owned coroutine from a Lua or native function.
    fn create_coroutine(&mut self, _function: Value) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support coroutine creation".into(),
        }
        .into())
    }

    /// Creates a callable wrapper around a new runtime-owned coroutine.
    fn create_coroutine_wrapper(&mut self, _function: Value) -> Result<Value, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support coroutine wrappers".into(),
        }
        .into())
    }

    /// Closes a runtime-owned coroutine.
    fn close_coroutine(&mut self, _thread: Value) -> Result<Result<(), Box<str>>, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support coroutine close".into(),
        }
        .into())
    }

    /// Resumes a runtime-owned coroutine with Lua values.
    fn resume_coroutine(
        &mut self,
        _thread: Value,
        _args: &[Value],
    ) -> Result<Result<Vec<Value>, Box<str>>, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support coroutine resume".into(),
        }
        .into())
    }

    /// Yields values from the currently running coroutine.
    fn yield_coroutine(&mut self, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support coroutine yield".into(),
        }
        .into())
    }

    /// Returns the currently running Lua thread and whether it is the main thread.
    fn running_thread(&self) -> Result<(Value, bool), NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support current coroutine lookup".into(),
        }
        .into())
    }

    /// Returns whether a runtime-owned Lua thread can yield.
    fn thread_is_yieldable(&self, _thread: Value) -> Result<bool, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support coroutine yieldability".into(),
        }
        .into())
    }

    /// Returns the status of a runtime-owned Lua thread.
    fn thread_status(&self, _thread: Value) -> Result<ThreadStatus, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support coroutine status".into(),
        }
        .into())
    }
}

/// Executable standard-library native function.
pub type NativeStdFunction = fn(&mut dyn NativeRuntime, &[Value]) -> NativeResult;

/// Descriptor plus executable implementation for one native function.
#[derive(Clone, Copy, Debug)]
pub struct NativeFunctionSpec {
    descriptor: FunctionSpec,
    function: NativeStdFunction,
}

impl NativeFunctionSpec {
    /// Creates an executable native function descriptor.
    #[must_use]
    pub const fn new(descriptor: FunctionSpec, function: NativeStdFunction) -> Self {
        Self {
            descriptor,
            function,
        }
    }

    /// Descriptor used by registry/profile code.
    #[must_use]
    pub const fn descriptor(self) -> FunctionSpec {
        self.descriptor
    }

    /// Native implementation.
    #[must_use]
    pub const fn function(self) -> NativeStdFunction {
        self.function
    }
}

impl PartialEq for NativeFunctionSpec {
    fn eq(&self, other: &Self) -> bool {
        self.descriptor == other.descriptor
    }
}

impl Eq for NativeFunctionSpec {}

/// Error raised by an executable standard-library native.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeError {
    kind: NativeErrorKind,
    message: Box<str>,
}

impl NativeError {
    /// Creates a native error from a stable kind.
    #[must_use]
    pub fn new(kind: NativeErrorKind) -> Self {
        let message = kind.message().into_boxed_str();
        Self { kind, message }
    }

    /// Creates a Lua-level error with a custom message payload.
    #[must_use]
    pub fn lua_error(message: impl Into<Box<str>>) -> Self {
        Self {
            kind: NativeErrorKind::LuaError,
            message: message.into(),
        }
    }

    /// Stable error kind.
    #[must_use]
    pub const fn kind(&self) -> &NativeErrorKind {
        &self.kind
    }

    /// Human-readable error message.
    #[must_use]
    pub const fn message(&self) -> &str {
        &self.message
    }
}

impl From<NativeErrorKind> for NativeError {
    fn from(kind: NativeErrorKind) -> Self {
        Self::new(kind)
    }
}

/// Stable native standard-library error kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeErrorKind {
    /// A required Lua argument was not provided.
    MissingArgument { index: usize },
    /// A Lua argument had the wrong type.
    TypeError {
        index: usize,
        expected: &'static str,
    },
    /// A Lua argument was outside the accepted range.
    ArgumentOutOfRange { index: usize },
    /// The native raised a Lua-level error.
    LuaError,
    /// Runtime service used by the native failed.
    RuntimeError { message: Box<str> },
}

impl NativeErrorKind {
    fn message(&self) -> String {
        match self {
            Self::MissingArgument { index } => format!("argument {index} expected"),
            Self::TypeError { index, expected } => {
                format!("bad argument #{index} ({expected} expected)")
            }
            Self::ArgumentOutOfRange { index } => {
                format!("bad argument #{index} (out of range)")
            }
            Self::LuaError => "native function error".to_owned(),
            Self::RuntimeError { message } => message.to_string(),
        }
    }
}
