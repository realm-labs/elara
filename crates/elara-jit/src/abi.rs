//! Baseline JIT ABI and runtime helper registry.

use std::{error::Error, fmt};

/// Status code returned by generated JIT functions and runtime helpers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JitStatus {
    /// Execution completed and results are available through the runtime context.
    Returned = 0,
    /// Execution should continue or restart in the interpreter.
    Fallback = 1,
    /// A runtime error was raised and stored on the runtime context.
    RuntimeError = 2,
    /// The JIT reached a bytecode path it does not currently support.
    Unsupported = 3,
    /// Execution yielded and yielded values are available through the context.
    Yielded = 4,
}

/// Opaque runtime context pointer passed between generated code and helpers.
#[repr(C)]
#[derive(Debug)]
pub struct JitRuntimeContext {
    _private: [u8; 0],
}

/// Native ABI used by generated JIT functions.
///
/// Calling generated code is unsafe because the function pointer is produced at
/// runtime and must match this exact ABI.
pub type JitFn = unsafe extern "C" fn(*mut JitRuntimeContext) -> JitStatus;

/// Native ABI used by runtime helper functions callable from generated code.
pub type RuntimeHelperFn = extern "C" fn(*mut JitRuntimeContext) -> JitStatus;

/// Stable identifier for a registered runtime helper.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeHelperId(u32);

impl RuntimeHelperId {
    /// Returns this helper id as a dense zero-based index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Registered runtime helper metadata.
#[derive(Clone, Copy, Debug)]
pub struct RuntimeHelper {
    name: &'static str,
    function: RuntimeHelperFn,
}

impl RuntimeHelper {
    /// Creates runtime helper metadata.
    #[must_use]
    pub const fn new(name: &'static str, function: RuntimeHelperFn) -> Self {
        Self { name, function }
    }

    /// Symbolic helper name used by lowering and module declaration.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// ABI function pointer for this helper.
    #[must_use]
    pub const fn function(self) -> RuntimeHelperFn {
        self.function
    }
}

/// Runtime helper registry used by the JIT lowering and runtime bridge.
#[derive(Clone, Debug, Default)]
pub struct RuntimeHelperRegistry {
    helpers: Vec<RuntimeHelper>,
}

impl RuntimeHelperRegistry {
    /// Creates an empty helper registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            helpers: Vec::new(),
        }
    }

    /// Registers a helper and returns its stable id.
    pub fn register(&mut self, helper: RuntimeHelper) -> RuntimeHelperId {
        let index = u32::try_from(self.helpers.len()).expect("runtime helper registry overflow");
        self.helpers.push(helper);
        RuntimeHelperId(index)
    }

    /// Returns helper metadata for an id.
    #[must_use]
    pub fn get(&self, id: RuntimeHelperId) -> Option<RuntimeHelper> {
        self.helpers.get(id.0 as usize).copied()
    }

    /// Returns the number of registered helpers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.helpers.len()
    }

    /// Returns true when no helpers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.helpers.is_empty()
    }

    /// Calls a registered helper directly.
    ///
    /// This supports ABI tests before M16.3 starts generating Lua code.
    pub fn call(
        &self,
        id: RuntimeHelperId,
        context: *mut JitRuntimeContext,
    ) -> Result<JitStatus, RuntimeHelperError> {
        let helper = self
            .get(id)
            .ok_or(RuntimeHelperError::MissingHelper { id })?;
        Ok((helper.function())(context))
    }
}

/// Runtime helper lookup or invocation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeHelperError {
    /// The requested helper id is not registered.
    MissingHelper {
        /// Missing helper id.
        id: RuntimeHelperId,
    },
}

impl fmt::Display for RuntimeHelperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHelper { id } => {
                write!(f, "missing JIT runtime helper {}", id.index())
            }
        }
    }
}

impl Error for RuntimeHelperError {}

#[cfg(test)]
mod tests {
    use std::ptr;

    use super::{
        JitRuntimeContext, JitStatus, RuntimeHelper, RuntimeHelperError, RuntimeHelperId,
        RuntimeHelperRegistry,
    };

    extern "C" fn return_helper(_context: *mut JitRuntimeContext) -> JitStatus {
        JitStatus::Returned
    }

    extern "C" fn fallback_helper(_context: *mut JitRuntimeContext) -> JitStatus {
        JitStatus::Fallback
    }

    #[test]
    fn abi_status_codes_are_stable() {
        assert_eq!(JitStatus::Returned as u32, 0);
        assert_eq!(JitStatus::Fallback as u32, 1);
        assert_eq!(JitStatus::RuntimeError as u32, 2);
        assert_eq!(JitStatus::Unsupported as u32, 3);
        assert_eq!(JitStatus::Yielded as u32, 4);
    }

    #[test]
    fn abi_runtime_helper_registry_calls_helpers_without_generated_code() {
        let mut registry = RuntimeHelperRegistry::new();
        let returned = registry.register(RuntimeHelper::new("return", return_helper));
        let fallback = registry.register(RuntimeHelper::new("fallback", fallback_helper));

        assert_eq!(returned.index(), 0);
        assert_eq!(fallback.index(), 1);
        assert_eq!(registry.len(), 2);
        assert_eq!(
            registry.call(returned, ptr::null_mut()),
            Ok(JitStatus::Returned)
        );
        assert_eq!(
            registry.call(fallback, ptr::null_mut()),
            Ok(JitStatus::Fallback)
        );
    }

    #[test]
    fn abi_runtime_helper_registry_reports_missing_helpers() {
        let registry = RuntimeHelperRegistry::new();
        let id = RuntimeHelperId(7);

        assert_eq!(
            registry.call(id, ptr::null_mut()),
            Err(RuntimeHelperError::MissingHelper { id })
        );
    }
}
