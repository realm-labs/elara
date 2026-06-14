//! Native Rust function support for the public API.

use std::{fmt, sync::Arc};

use elara_core::Value;
use elara_interp::{NativeContext, RuntimeError, RuntimeErrorKind, RuntimeResult};

use crate::{ConversionError, FromLuaMulti, IntoLuaMulti, LuaValue};

/// Error returned by native Rust callbacks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeFunctionError {
    message: Box<str>,
}

impl NativeFunctionError {
    /// Creates a native function error with a Lua-facing message.
    #[must_use]
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Lua-facing error message.
    #[must_use]
    pub const fn message(&self) -> &str {
        &self.message
    }
}

impl From<ConversionError> for NativeFunctionError {
    fn from(error: ConversionError) -> Self {
        Self::new(format!(
            "bad native argument ({} expected, got {})",
            error.expected(),
            error.actual()
        ))
    }
}

/// Callable native Rust function handle.
#[derive(Clone)]
pub struct Function {
    inner: Arc<NativeCallback>,
}

type NativeCallback =
    dyn for<'a> Fn(&mut NativeContext<'a>, &[Value]) -> RuntimeResult<Vec<Value>> + Send + Sync;

impl Function {
    /// Creates a typed native Rust function.
    pub fn new<Args, Returns, F>(function: F) -> Self
    where
        Args: FromLuaMulti,
        Returns: IntoLuaMulti,
        F: Fn(Args) -> Result<Returns, NativeFunctionError> + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(move |context, args| {
                let lua_args = args
                    .iter()
                    .copied()
                    .map(|value| runtime_value_to_lua(context, value))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(native_error_to_runtime_error)?;
                let typed_args = Args::from_lua_multi(&lua_args)
                    .map_err(NativeFunctionError::from)
                    .map_err(native_error_to_runtime_error)?;
                let returns = function(typed_args).map_err(native_error_to_runtime_error)?;
                returns
                    .into_lua_multi()
                    .map_err(NativeFunctionError::from)
                    .and_then(|values| {
                        values
                            .into_iter()
                            .map(|value| lua_value_to_runtime(context, value))
                            .collect()
                    })
                    .map_err(native_error_to_runtime_error)
            }),
        }
    }

    pub(crate) fn call(
        &self,
        context: &mut NativeContext<'_>,
        args: &[Value],
    ) -> RuntimeResult<Vec<Value>> {
        (self.inner)(context, args)
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Function").finish_non_exhaustive()
    }
}

fn runtime_value_to_lua(
    context: &NativeContext<'_>,
    value: Value,
) -> Result<LuaValue, NativeFunctionError> {
    if value.is_nil() {
        return Ok(LuaValue::Nil);
    }
    if let Some(value) = value.as_bool() {
        return Ok(LuaValue::Boolean(value));
    }
    if let Some(value) = value.as_integer() {
        return Ok(LuaValue::Integer(value));
    }
    if let Some(value) = value.as_float() {
        return Ok(LuaValue::Number(value));
    }
    if value.is_string() {
        let bytes = context
            .short_string_bytes(value)
            .ok_or_else(|| NativeFunctionError::new("unsupported runtime string value"))?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| NativeFunctionError::new("non-utf8 Lua string"))?;
        return Ok(LuaValue::String(text.into()));
    }
    Err(NativeFunctionError::new(format!(
        "unsupported native argument type {}",
        value.tag() as u8
    )))
}

fn lua_value_to_runtime(
    context: &mut NativeContext<'_>,
    value: LuaValue,
) -> Result<Value, NativeFunctionError> {
    match value {
        LuaValue::Nil => Ok(Value::nil()),
        LuaValue::Boolean(value) => Ok(Value::boolean(value)),
        LuaValue::Integer(value) => Ok(Value::integer(value)),
        LuaValue::Number(value) => Ok(Value::float(value)),
        LuaValue::String(value) => context
            .intern_short_string(value.as_bytes())
            .map_err(|error| NativeFunctionError::new(error.message().to_owned())),
    }
}

fn native_error_to_runtime_error(error: NativeFunctionError) -> RuntimeError {
    RuntimeErrorKind::NativeFunctionError {
        message: error.message,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use elara_core::{SourceId, Value};
    use elara_stdlib::{StdLib, StdLibProfile};

    use crate::{Lua, NativeFunctionError, eval_simple_source_with_stdlib};

    #[test]
    fn native_function_can_be_called_from_chunk() {
        let lua = Lua::new();
        let add = lua.create_function(|(left, right): (i64, i64)| Ok((left + right,)));
        lua.set_global_function("add", add);

        assert_eq!(lua.eval("return add(20, 22)"), Ok(vec![Value::integer(42)]));
    }

    #[test]
    fn native_function_extracts_string_arguments_and_returns_string() {
        let lua = Lua::new();
        let greet = lua.create_function(|(name,): (String,)| Ok((format!("hi {name}"),)));
        lua.set_global_function("greet", greet);

        assert_eq!(
            lua.eval("return string.len(greet('Lua'))"),
            Ok(vec![Value::integer(6)])
        );
    }

    #[test]
    fn native_function_reports_argument_conversion_errors() {
        let lua = Lua::new();
        let add = lua.create_function(|(left, right): (i64, i64)| Ok((left + right,)));
        lua.set_global_function("add", add);

        assert!(lua.eval("return add('bad', 1)").is_err());
    }

    #[test]
    fn native_function_converts_callback_errors() {
        let lua = Lua::new();
        let fail = lua.create_function(|(): ()| -> Result<(i64,), NativeFunctionError> {
            Err(NativeFunctionError::new("boom"))
        });
        lua.set_global_function("fail", fail);

        assert!(lua.eval("return fail()").is_err());
    }

    #[test]
    fn native_function_does_not_affect_plain_stdlib_eval_path() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.abs(-3)", &profile),
            Ok(vec![Value::integer(3)])
        );
    }
}
