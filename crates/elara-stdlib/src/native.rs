//! Executable native standard-library function descriptors.

use elara_core::Value;

use crate::FunctionSpec;

/// Result returned by executable standard-library natives.
pub type NativeResult = Result<Vec<Value>, NativeError>;

/// Executable standard-library native function.
pub type NativeStdFunction = fn(&[Value]) -> NativeResult;

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
}

impl NativeErrorKind {
    fn message(&self) -> String {
        match self {
            Self::MissingArgument { index } => format!("argument {index} expected"),
            Self::TypeError { index, expected } => {
                format!("bad argument #{index} ({expected} expected)")
            }
        }
    }
}
