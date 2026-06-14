//! JIT call trampoline helpers.

use std::ptr::NonNull;

use elara_core::Value;

use crate::{JitRuntimeContext, JitStatus, RuntimeHelper};

/// Symbol name used for the call trampoline helper.
pub const JIT_CALL_HELPER_NAME: &str = "elara_jit_call";

/// Fallback invoked when generated code calls a runtime native function.
pub type NativeCallFallback<'a> = dyn Fn(usize, &[Value]) -> JitCallOutcome + 'a;

/// Fallback invoked when generated code calls a Lua closure.
pub type LuaCallFallback<'a> = dyn Fn(usize, &[Value]) -> JitCallOutcome + 'a;

/// Call request materialized by generated code before entering the trampoline.
#[derive(Clone, Debug, PartialEq)]
pub struct JitCallRequest {
    callee: Value,
    args: Vec<Value>,
    result_count: u32,
}

impl JitCallRequest {
    /// Creates a call request.
    #[must_use]
    pub fn new(callee: Value, args: impl Into<Vec<Value>>, result_count: u32) -> Self {
        Self {
            callee,
            args: args.into(),
            result_count,
        }
    }

    /// Callee value.
    #[must_use]
    pub const fn callee(&self) -> Value {
        self.callee
    }

    /// Positional call arguments.
    #[must_use]
    pub fn args(&self) -> &[Value] {
        &self.args
    }

    /// Requested return count encoded by the bytecode call instruction.
    #[must_use]
    pub const fn result_count(&self) -> u32 {
        self.result_count
    }
}

/// Result from a Lua or native fallback call.
#[derive(Clone, Debug, PartialEq)]
pub enum JitCallOutcome {
    /// Call returned normally.
    Returned(Vec<Value>),
    /// Call yielded from a resumable path.
    Yielded(Vec<Value>),
    /// Call raised a runtime error.
    RuntimeError(Box<str>),
}

impl JitCallOutcome {
    fn into_result(self) -> JitCallResult {
        match self {
            Self::Returned(values) => JitCallResult::returned(values),
            Self::Yielded(values) => JitCallResult::yielded(values),
            Self::RuntimeError(message) => JitCallResult::runtime_error(message),
        }
    }
}

/// Materialized call result left in the runtime context after the helper returns.
#[derive(Clone, Debug, PartialEq)]
pub struct JitCallResult {
    status: JitStatus,
    values: Vec<Value>,
    error: Option<Box<str>>,
}

impl JitCallResult {
    /// Creates a normal-return result.
    #[must_use]
    pub fn returned(values: impl Into<Vec<Value>>) -> Self {
        Self {
            status: JitStatus::Returned,
            values: values.into(),
            error: None,
        }
    }

    /// Creates a yielded result.
    #[must_use]
    pub fn yielded(values: impl Into<Vec<Value>>) -> Self {
        Self {
            status: JitStatus::Yielded,
            values: values.into(),
            error: None,
        }
    }

    /// Creates a runtime-error result.
    #[must_use]
    pub fn runtime_error(message: impl Into<Box<str>>) -> Self {
        Self {
            status: JitStatus::RuntimeError,
            values: Vec::new(),
            error: Some(message.into()),
        }
    }

    /// Creates an unsupported-call result.
    #[must_use]
    pub fn unsupported(message: impl Into<Box<str>>) -> Self {
        Self {
            status: JitStatus::Unsupported,
            values: Vec::new(),
            error: Some(message.into()),
        }
    }

    /// Status returned by the trampoline.
    #[must_use]
    pub const fn status(&self) -> JitStatus {
        self.status
    }

    /// Returned or yielded values.
    #[must_use]
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Runtime error or unsupported-call reason.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl Default for JitCallResult {
    fn default() -> Self {
        Self::unsupported("call trampoline has not run")
    }
}

/// Safe call router used by the helper and by focused tests.
#[derive(Default)]
pub struct JitCallTrampoline<'a> {
    native_fallback: Option<&'a NativeCallFallback<'a>>,
    lua_fallback: Option<&'a LuaCallFallback<'a>>,
}

impl<'a> JitCallTrampoline<'a> {
    /// Creates a trampoline with no fallback handlers.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            native_fallback: None,
            lua_fallback: None,
        }
    }

    /// Installs the native-call fallback handler.
    #[must_use]
    pub const fn with_native_fallback(mut self, fallback: &'a NativeCallFallback<'a>) -> Self {
        self.native_fallback = Some(fallback);
        self
    }

    /// Installs the Lua-call fallback handler.
    #[must_use]
    pub const fn with_lua_fallback(mut self, fallback: &'a LuaCallFallback<'a>) -> Self {
        self.lua_fallback = Some(fallback);
        self
    }

    /// Routes a generated-code call request to the correct fallback path.
    #[must_use]
    pub fn route(&self, request: &JitCallRequest) -> JitCallResult {
        if let Some(native_index) = request.callee().as_native_function_index() {
            return self.call_native(native_index as usize, request.args());
        }
        if let Some(closure_index) = request.callee().as_closure_index() {
            return self.call_lua(closure_index as usize, request.args());
        }
        JitCallResult::runtime_error("attempt to call a non-function value")
    }

    fn call_native(&self, native_index: usize, args: &[Value]) -> JitCallResult {
        let Some(fallback) = self.native_fallback else {
            return JitCallResult::unsupported("native call fallback is not installed");
        };
        fallback(native_index, args).into_result()
    }

    fn call_lua(&self, closure_index: usize, args: &[Value]) -> JitCallResult {
        let Some(fallback) = self.lua_fallback else {
            return JitCallResult::unsupported("Lua call fallback is not installed");
        };
        fallback(closure_index, args).into_result()
    }
}

/// Raw context layout consumed by [`jit_call_helper`].
#[repr(C)]
pub struct JitCallRuntimeContext<'a> {
    trampoline: *const JitCallTrampoline<'a>,
    request: *const JitCallRequest,
    result: *mut JitCallResult,
}

impl<'a> JitCallRuntimeContext<'a> {
    /// Creates a helper context from safe references.
    pub fn new(
        trampoline: &'a JitCallTrampoline<'a>,
        request: &'a JitCallRequest,
        result: &'a mut JitCallResult,
    ) -> Self {
        Self {
            trampoline: std::ptr::from_ref(trampoline),
            request: std::ptr::from_ref(request),
            result: NonNull::from(result).as_ptr(),
        }
    }
}

/// Returns runtime-helper metadata for the JIT call trampoline.
#[must_use]
pub const fn jit_call_runtime_helper() -> RuntimeHelper {
    RuntimeHelper::new(JIT_CALL_HELPER_NAME, jit_call_helper)
}

/// Runtime helper ABI entry point for generated-code calls.
pub extern "C" fn jit_call_helper(context: *mut JitRuntimeContext) -> JitStatus {
    let Some(context) = NonNull::new(context.cast::<JitCallRuntimeContext<'_>>()) else {
        return JitStatus::RuntimeError;
    };
    let context = {
        // SAFETY: The caller passes a `JitCallRuntimeContext` pointer through the
        // opaque JIT context ABI. Null was checked above.
        unsafe { context.as_ref() }
    };
    let Some(trampoline) = NonNull::new(context.trampoline.cast_mut()) else {
        return JitStatus::RuntimeError;
    };
    let Some(request) = NonNull::new(context.request.cast_mut()) else {
        return JitStatus::RuntimeError;
    };
    let Some(mut result) = NonNull::new(context.result) else {
        return JitStatus::RuntimeError;
    };

    let routed = {
        // SAFETY: All pointers come from `JitCallRuntimeContext::new` or an
        // equivalent generated-code context and were checked for null above.
        let trampoline = unsafe { trampoline.as_ref() };
        let request = unsafe { request.as_ref() };
        trampoline.route(request)
    };
    let status = routed.status();
    // SAFETY: `result` points to writable storage supplied by the runtime context.
    unsafe {
        *result.as_mut() = routed;
    }
    status
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{
        JIT_CALL_HELPER_NAME, JitCallOutcome, JitCallRequest, JitCallResult, JitCallRuntimeContext,
        JitCallTrampoline, jit_call_runtime_helper,
    };
    use crate::{JitRuntimeContext, JitStatus, RuntimeHelperRegistry};

    #[test]
    fn calls_trampoline_routes_native_call_fallback() {
        let native = |index, args: &[Value]| {
            assert_eq!(index, 7);
            JitCallOutcome::Returned(vec![Value::integer(
                args[0].as_integer().unwrap() + args[1].as_integer().unwrap(),
            )])
        };
        let trampoline = JitCallTrampoline::new().with_native_fallback(&native);
        let request = JitCallRequest::new(
            Value::native_function_index(7),
            [Value::integer(2), Value::integer(3)],
            1,
        );

        let result = trampoline.route(&request);

        assert_eq!(result.status(), JitStatus::Returned);
        assert_eq!(result.values(), &[Value::integer(5)]);
        assert_eq!(result.error(), None);
    }

    #[test]
    fn calls_trampoline_routes_lua_call_fallback() {
        let lua = |index, args: &[Value]| {
            assert_eq!(index, 3);
            assert_eq!(args, &[Value::integer(9)]);
            JitCallOutcome::Returned(vec![Value::integer(42)])
        };
        let trampoline = JitCallTrampoline::new().with_lua_fallback(&lua);
        let request = JitCallRequest::new(Value::closure_index(3), [Value::integer(9)], 1);

        let result = trampoline.route(&request);

        assert_eq!(result.status(), JitStatus::Returned);
        assert_eq!(result.values(), &[Value::integer(42)]);
    }

    #[test]
    fn calls_trampoline_preserves_yield_and_error_statuses() {
        let lua = |_index, _args: &[Value]| JitCallOutcome::Yielded(vec![Value::integer(11)]);
        let native = |_index, _args: &[Value]| JitCallOutcome::RuntimeError("boom".into());
        let trampoline = JitCallTrampoline::new()
            .with_lua_fallback(&lua)
            .with_native_fallback(&native);

        let yielded = trampoline.route(&JitCallRequest::new(Value::closure_index(0), [], 0));
        let errored =
            trampoline.route(&JitCallRequest::new(Value::native_function_index(0), [], 0));

        assert_eq!(yielded.status(), JitStatus::Yielded);
        assert_eq!(yielded.values(), &[Value::integer(11)]);
        assert_eq!(errored.status(), JitStatus::RuntimeError);
        assert_eq!(errored.error(), Some("boom"));
    }

    #[test]
    fn calls_trampoline_reports_unsupported_or_non_callable_paths() {
        let trampoline = JitCallTrampoline::new();

        let missing_native =
            trampoline.route(&JitCallRequest::new(Value::native_function_index(0), [], 0));
        let non_callable = trampoline.route(&JitCallRequest::new(Value::integer(1), [], 0));

        assert_eq!(missing_native.status(), JitStatus::Unsupported);
        assert_eq!(
            missing_native.error(),
            Some("native call fallback is not installed")
        );
        assert_eq!(non_callable.status(), JitStatus::RuntimeError);
        assert_eq!(
            non_callable.error(),
            Some("attempt to call a non-function value")
        );
    }

    #[test]
    fn calls_runtime_helper_invokes_trampoline_from_opaque_context() {
        let native = |_index, args: &[Value]| JitCallOutcome::Returned(args.to_vec());
        let trampoline = JitCallTrampoline::new().with_native_fallback(&native);
        let request = JitCallRequest::new(Value::native_function_index(2), [Value::integer(8)], 1);
        let mut result = JitCallResult::default();
        let mut context = JitCallRuntimeContext::new(&trampoline, &request, &mut result);
        let mut registry = RuntimeHelperRegistry::new();
        let helper = jit_call_runtime_helper();
        let helper_id = registry.register(helper);

        assert_eq!(helper.name(), JIT_CALL_HELPER_NAME);
        assert_eq!(
            registry.call(
                helper_id,
                (&mut context as *mut JitCallRuntimeContext<'_>).cast::<JitRuntimeContext>(),
            ),
            Ok(JitStatus::Returned)
        );
        assert_eq!(result.status(), JitStatus::Returned);
        assert_eq!(result.values(), &[Value::integer(8)]);
    }
}
