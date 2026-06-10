//! Executable native standard-library function descriptors.

use elara_core::Value;

use crate::FunctionSpec;

/// Result returned by executable standard-library natives.
pub type NativeResult = Result<Vec<Value>, NativeError>;

/// Runtime services available to standard-library native functions.
pub trait NativeRuntime {
    /// Interns a short Lua string in the current runtime.
    fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError>;

    /// Returns bytes for a short Lua string owned by this runtime.
    fn short_string_bytes(&self, value: Value) -> Option<&[u8]>;

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

    /// Returns the next 64 bits from this runtime's standard-library RNG.
    fn next_random_u64(&mut self) -> Result<u64, NativeError> {
        Err(NativeErrorKind::RuntimeError {
            message: "native runtime does not support random numbers".into(),
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
