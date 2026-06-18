//! Executable base-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
    native::protected_error_message,
    number::{parse_base_integer, parse_standard_number},
};

/// Executable base-library functions currently implemented.
pub const BASE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "assert"), base_assert),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Base, "collectgarbage"),
        base_collectgarbage,
    ),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "dofile"), base_dofile),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "error"), base_error),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Base, "getmetatable"),
        base_getmetatable,
    ),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "ipairs"), base_ipairs),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "load"), base_load),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "loadfile"), base_loadfile),
    BASE_NEXT_NATIVE,
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "pairs"), base_pairs),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "pcall"), base_pcall),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "print"), base_print),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawequal"), base_rawequal),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawget"), base_rawget),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawlen"), base_rawlen),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawset"), base_rawset),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "select"), base_select),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Base, "setmetatable"),
        base_setmetatable,
    ),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "tonumber"), base_tonumber),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "tostring"), base_tostring),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "type"), base_type),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "warn"), base_warn),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "xpcall"), base_xpcall),
];

const UNSUPPORTED_DYNAMIC_LOADING: &str = "dynamic Lua loading is not supported";
const UNSUPPORTED_GC_CONTROL: &str = "collectgarbage is not supported";

/// Hidden helper used by `ipairs`.
pub const BASE_IPAIRS_AUX_NATIVE: NativeFunctionSpec = NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Base, "__ipairs_aux"),
    base_ipairs_aux,
);

/// Raw table traversal helper used by `next` and `pairs`.
pub const BASE_NEXT_NATIVE: NativeFunctionSpec =
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "next"), base_next);

fn base_assert(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let condition = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if is_truthy(condition) {
        Ok(args.to_vec())
    } else {
        match args.get(1).copied() {
            Some(value) => Err(NativeError::lua_error_object(
                value,
                error_message(runtime, value),
            )),
            None => Err(NativeError::lua_error("assertion failed!")),
        }
    }
}

fn base_collectgarbage(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    validate_optional_string(runtime, args, 1)?;
    Err(NativeError::lua_error(UNSUPPORTED_GC_CONTROL))
}

fn base_dofile(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    validate_optional_string(runtime, args, 1)?;
    Err(NativeError::lua_error(UNSUPPORTED_DYNAMIC_LOADING))
}

fn base_error(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    if let Some(level) = args.get(1).filter(|value| !value.is_nil()) {
        level.as_integer().ok_or(NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        })?;
    }
    let value = args.first().copied().unwrap_or_else(Value::nil);
    Err(NativeError::lua_error_object(
        value,
        error_message(runtime, value),
    ))
}

fn base_getmetatable(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !value.is_table() {
        return Ok(vec![Value::nil()]);
    }
    let metatable = runtime.table_metatable(value)?;
    if metatable.is_nil() {
        return Ok(vec![Value::nil()]);
    }
    let metatable_key = metatable_field_key(runtime)?;
    let protected = runtime.table_get(metatable, metatable_key)?;
    if protected.is_nil() {
        Ok(vec![metatable])
    } else {
        Ok(vec![protected])
    }
}

fn base_ipairs(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    Ok(vec![
        runtime.native_function(StdLib::Base, "__ipairs_aux")?,
        value,
        Value::integer(0),
    ])
}

fn base_ipairs_aux(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let table = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let index = args
        .get(1)
        .and_then(|value| value.as_integer())
        .ok_or(NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        })?
        .checked_add(1)
        .ok_or(NativeErrorKind::ArgumentOutOfRange { index: 2 })?;
    let value = runtime.table_get_integer(table, index)?;
    if value.is_nil() {
        Ok(vec![Value::nil()])
    } else {
        Ok(vec![Value::integer(index), value])
    }
}

fn base_load(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let chunk = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if runtime.string_bytes(chunk).is_none() && !is_function(chunk) {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "string or function",
        }
        .into());
    }
    validate_optional_string(runtime, args, 2)?;
    validate_optional_mode(runtime, args, 3)?;
    unsupported_loading_result(runtime)
}

fn base_loadfile(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    validate_optional_string(runtime, args, 1)?;
    validate_optional_mode(runtime, args, 2)?;
    unsupported_loading_result(runtime)
}

fn base_next(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let table = table_arg(args, 1)?;
    let key = args.get(1).copied().unwrap_or_else(Value::nil);
    Ok(runtime
        .table_next(table, key)?
        .map_or_else(|| vec![Value::nil()], |(key, value)| vec![key, value]))
}

fn base_pairs(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if let Some(metamethod) = pairs_metamethod(runtime, value)? {
        return call_pairs_metamethod(runtime, metamethod, value);
    }
    Ok(vec![
        runtime.native_function(StdLib::Base, "next")?,
        value,
        Value::nil(),
        Value::nil(),
    ])
}

fn base_pcall(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let function = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let call_args = args.get(1..).unwrap_or_default();
    match runtime.protected_call(function, call_args)? {
        Ok(values) => {
            let mut results = Vec::with_capacity(values.len() + 1);
            results.push(Value::boolean(true));
            results.extend(values);
            Ok(results)
        }
        Err(error) => Ok(vec![Value::boolean(false), error]),
    }
}

fn base_xpcall(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let function = args.first().copied().unwrap_or_else(Value::nil);
    let handler = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    if !is_function(handler) {
        return Err(NativeErrorKind::TypeError {
            index: 2,
            expected: "function",
        }
        .into());
    }
    let call_args = args.get(2..).unwrap_or_default();
    match runtime.protected_call(function, call_args)? {
        Ok(values) => {
            let mut results = Vec::with_capacity(values.len() + 1);
            results.push(Value::boolean(true));
            results.extend(values);
            Ok(results)
        }
        Err(error) => match runtime.protected_call(handler, &[error])? {
            Ok(values) => {
                let mut results = Vec::with_capacity(values.len() + 1);
                results.push(Value::boolean(false));
                results.extend(values);
                Ok(results)
            }
            Err(handler_error) => Ok(vec![Value::boolean(false), handler_error]),
        },
    }
}

fn base_print(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    for (index, value) in args.iter().copied().enumerate() {
        if index > 0 {
            runtime.write_output(b"\t")?;
        }
        let bytes = printable_bytes(runtime, value)?;
        runtime.write_output(&bytes)?;
    }
    runtime.write_output(b"\n")?;
    Ok(Vec::new())
}

fn base_rawequal(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let left = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let right = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    Ok(vec![Value::boolean(left == right)])
}

fn base_rawget(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let table = table_arg(args, 1)?;
    let key = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    Ok(vec![runtime.table_get(table, key)?])
}

fn base_rawlen(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if let Some(bytes) = runtime.string_bytes(value) {
        let len = i64::try_from(bytes.len()).expect("runtime string length must fit in LuaInteger");
        return Ok(vec![Value::integer(len)]);
    }
    if value.is_table() {
        return Ok(vec![Value::integer(runtime.table_array_len(value)?)]);
    }
    Err(NativeErrorKind::TypeError {
        index: 1,
        expected: "table or string",
    }
    .into())
}

fn base_rawset(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let table = table_arg(args, 1)?;
    let key = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    let value = *args
        .get(2)
        .ok_or(NativeErrorKind::MissingArgument { index: 3 })?;
    runtime.table_set(table, key, value)?;
    Ok(vec![table])
}

fn base_select(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    if args
        .first()
        .and_then(|value| runtime.string_bytes(*value))
        .is_some_and(|bytes| bytes.first() == Some(&b'#'))
    {
        let count = i64::try_from(args.len().saturating_sub(1)).map_err(|_| {
            NativeErrorKind::RuntimeError {
                message: "too many arguments to select".into(),
            }
        })?;
        return Ok(vec![Value::integer(count)]);
    }

    let index = select_index(runtime, args.first().copied())?;
    let value_count = args.len().saturating_sub(1);
    let start = select_start(index, value_count)?;
    Ok(args[start..].to_vec())
}

fn base_setmetatable(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let table = table_arg(args, 1)?;
    let metatable = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    if !metatable.is_nil() && !metatable.is_table() {
        return Err(NativeErrorKind::TypeError {
            index: 2,
            expected: "nil or table",
        }
        .into());
    }
    let current = runtime.table_metatable(table)?;
    if !current.is_nil() {
        let metatable_key = metatable_field_key(runtime)?;
        let protected = runtime.table_get(current, metatable_key)?;
        if !protected.is_nil() {
            return Err(NativeError::lua_error(
                "cannot change a protected metatable",
            ));
        }
    }
    runtime.table_set_metatable(table, metatable)?;
    Ok(vec![table])
}

fn base_tonumber(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let Some(base) = args.get(1).filter(|value| !value.is_nil()) else {
        if value.is_number() {
            return Ok(vec![value]);
        }
        return Ok(vec![
            runtime
                .string_bytes(value)
                .and_then(parse_standard_number)
                .unwrap_or_else(Value::nil),
        ]);
    };

    let base = base.as_integer().ok_or(NativeErrorKind::TypeError {
        index: 2,
        expected: "integer",
    })?;
    if !(2..=36).contains(&base) {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 2 }.into());
    }
    let bytes = runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index: 1,
            expected: "string",
        })?;
    Ok(vec![
        parse_base_integer(bytes, base as u32).map_or_else(Value::nil, Value::integer),
    ])
}

fn base_tostring(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if runtime.string_bytes(value).is_some() {
        return Ok(vec![value]);
    }
    let bytes = tostring_output_bytes(runtime, value)?;
    Ok(vec![runtime.intern_string(&bytes)?])
}

fn base_type(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    Ok(vec![
        runtime.intern_short_string(type_name(value).as_bytes())?,
    ])
}

fn base_warn(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    if args.is_empty() {
        return Err(NativeErrorKind::MissingArgument { index: 1 }.into());
    }
    let mut parts = Vec::with_capacity(args.len());
    for index in 1..=args.len().max(1) {
        let value = args
            .get(index - 1)
            .ok_or(NativeErrorKind::MissingArgument { index })?;
        let bytes = runtime
            .string_bytes(*value)
            .ok_or(NativeErrorKind::TypeError {
                index,
                expected: "string",
            })?
            .to_vec();
        parts.push(bytes);
    }

    if let [control] = parts.as_slice()
        && control.first() == Some(&b'@')
    {
        match control.as_slice() {
            b"@on" => runtime.set_warnings_enabled(true)?,
            b"@off" => runtime.set_warnings_enabled(false)?,
            _ => {}
        }
        return Ok(Vec::new());
    }

    if runtime.warnings_enabled() {
        runtime.write_warning(b"Lua warning: ")?;
        for part in &parts {
            runtime.write_warning(part)?;
        }
        runtime.write_warning(b"\n")?;
    }
    Ok(Vec::new())
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

fn is_function(value: Value) -> bool {
    value.is_closure() || value.as_native_function_index().is_some()
}

fn type_name(value: Value) -> &'static str {
    if value.is_nil() {
        "nil"
    } else if value.is_bool() {
        "boolean"
    } else if value.is_number() {
        "number"
    } else if value.is_string() {
        "string"
    } else if value.is_table() {
        "table"
    } else if value.is_closure() {
        "function"
    } else if value.is_thread() {
        "thread"
    } else if value.is_light_user_data() {
        "userdata"
    } else {
        "unknown"
    }
}

fn tostring_bytes(value: Value) -> String {
    if value.is_nil() {
        "nil".to_owned()
    } else if let Some(value) = value.as_bool() {
        value.to_string()
    } else if let Some(value) = value.as_integer() {
        value.to_string()
    } else if let Some(value) = value.as_float() {
        value.to_string()
    } else if let Some(index) = value.as_table_index() {
        format!("table: 0x{index:x}")
    } else if let Some(index) = value.as_closure_index() {
        format!("function: 0x{index:x}")
    } else if let Some(index) = value.as_native_function_index() {
        format!("function: 0x{index:x}")
    } else if let Some(index) = value.as_thread_index() {
        format!("thread: 0x{index:x}")
    } else if let Some(value) = value.as_light_user_data() {
        format!("userdata: 0x{value:x}")
    } else {
        format!("{}: 0x0", type_name(value))
    }
}

fn tostring_output_bytes(
    runtime: &mut dyn NativeRuntime,
    value: Value,
) -> Result<Vec<u8>, NativeError> {
    if let Some(bytes) = runtime.string_bytes(value) {
        return Ok(bytes.to_vec());
    }
    if let Some(metamethod) = tostring_metamethod(runtime, value)? {
        return call_tostring_metamethod(runtime, metamethod, value);
    }
    if let Some(kind) = metatable_name(runtime, value)? {
        if let Some(index) = value.as_table_index() {
            return Ok(format!("{kind}: 0x{index:x}").into_bytes());
        }
    }
    Ok(tostring_bytes(value).into_bytes())
}

fn printable_bytes(runtime: &mut dyn NativeRuntime, value: Value) -> Result<Vec<u8>, NativeError> {
    tostring_output_bytes(runtime, value)
}

fn unsupported_loading_result(runtime: &mut dyn NativeRuntime) -> Result<Vec<Value>, NativeError> {
    Ok(vec![
        Value::nil(),
        runtime.intern_string(UNSUPPORTED_DYNAMIC_LOADING.as_bytes())?,
    ])
}

fn validate_optional_string(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<(), NativeError> {
    let Some(value) = args.get(index - 1).filter(|value| !value.is_nil()) else {
        return Ok(());
    };
    if runtime.string_bytes(*value).is_some() {
        Ok(())
    } else {
        Err(NativeErrorKind::TypeError {
            index,
            expected: "string",
        }
        .into())
    }
}

fn validate_optional_mode(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<(), NativeError> {
    let Some(value) = args.get(index - 1).filter(|value| !value.is_nil()) else {
        return Ok(());
    };
    let mode = runtime
        .string_bytes(*value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string",
        })?;
    if mode.contains(&b'B') {
        return Err(NativeError::lua_error("invalid mode"));
    }
    Ok(())
}

fn error_message(runtime: &dyn NativeRuntime, value: Value) -> String {
    if value.is_nil() {
        return "<no error object>".to_owned();
    }
    runtime.string_bytes(value).map_or_else(
        || tostring_bytes(value),
        |bytes| String::from_utf8_lossy(bytes).into_owned(),
    )
}

fn metatable_field_key(runtime: &mut dyn NativeRuntime) -> Result<Value, NativeError> {
    runtime.intern_short_string(b"__metatable")
}

fn metatable_name_key(runtime: &mut dyn NativeRuntime) -> Result<Value, NativeError> {
    runtime.intern_short_string(b"__name")
}

fn tostring_metamethod_key(runtime: &mut dyn NativeRuntime) -> Result<Value, NativeError> {
    runtime.intern_short_string(b"__tostring")
}

fn metatable_name(
    runtime: &mut dyn NativeRuntime,
    value: Value,
) -> Result<Option<String>, NativeError> {
    if !value.is_table() {
        return Ok(None);
    }
    let metatable = runtime.table_metatable(value)?;
    if metatable.is_nil() {
        return Ok(None);
    }
    let name_key = metatable_name_key(runtime)?;
    let name = runtime.table_get(metatable, name_key)?;
    let Some(bytes) = runtime.string_bytes(name) else {
        return Ok(None);
    };
    Ok(Some(String::from_utf8_lossy(bytes).into_owned()))
}

fn tostring_metamethod(
    runtime: &mut dyn NativeRuntime,
    value: Value,
) -> Result<Option<Value>, NativeError> {
    if !value.is_table() {
        return Ok(None);
    }
    let metatable = runtime.table_metatable(value)?;
    if metatable.is_nil() {
        return Ok(None);
    }
    let key = tostring_metamethod_key(runtime)?;
    let metamethod = runtime.table_get(metatable, key)?;
    Ok((!metamethod.is_nil()).then_some(metamethod))
}

fn call_tostring_metamethod(
    runtime: &mut dyn NativeRuntime,
    metamethod: Value,
    value: Value,
) -> Result<Vec<u8>, NativeError> {
    let results = runtime
        .protected_call(metamethod, &[value])?
        .map_err(|error| {
            NativeError::lua_error_object(error, protected_error_message(runtime, error))
        })?;
    let result = results.first().copied().unwrap_or_else(Value::nil);
    if let Some(bytes) = runtime.string_bytes(result) {
        Ok(bytes.to_vec())
    } else if result.is_number() {
        Ok(tostring_bytes(result).into_bytes())
    } else {
        Err(NativeError::lua_error("'__tostring' must return a string"))
    }
}

fn pairs_key(runtime: &mut dyn NativeRuntime) -> Result<Value, NativeError> {
    runtime.intern_short_string(b"__pairs")
}

fn pairs_metamethod(
    runtime: &mut dyn NativeRuntime,
    value: Value,
) -> Result<Option<Value>, NativeError> {
    if !value.is_table() {
        return Ok(None);
    }
    let metatable = runtime.table_metatable(value)?;
    if metatable.is_nil() {
        return Ok(None);
    }
    let key = pairs_key(runtime)?;
    let metamethod = runtime.table_get(metatable, key)?;
    Ok((!metamethod.is_nil()).then_some(metamethod))
}

fn call_pairs_metamethod(
    runtime: &mut dyn NativeRuntime,
    metamethod: Value,
    value: Value,
) -> Result<Vec<Value>, NativeError> {
    let mut results = runtime
        .protected_call(metamethod, &[value])?
        .map_err(|error| NativeError::lua_error(protected_error_message(runtime, error)))?;
    results.truncate(4);
    while results.len() < 4 {
        results.push(Value::nil());
    }
    Ok(results)
}

fn table_arg(args: &[Value], index: usize) -> Result<Value, NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    if value.is_table() {
        Ok(value)
    } else {
        Err(NativeErrorKind::TypeError {
            index,
            expected: "table",
        }
        .into())
    }
}

fn select_start(index: i64, value_count: usize) -> Result<usize, NativeError> {
    let value_count = i64::try_from(value_count).expect("argument count must fit in i64");
    let normalized = if index < 0 {
        value_count + index + 1
    } else if index > value_count {
        value_count + 1
    } else {
        index
    };
    if normalized < 1 {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 1 }.into());
    }
    usize::try_from(normalized).map_err(|_| NativeErrorKind::ArgumentOutOfRange { index: 1 }.into())
}

fn select_index(runtime: &dyn NativeRuntime, value: Option<Value>) -> Result<i64, NativeError> {
    let value = value.ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if let Some(integer) = value.as_integer() {
        return Ok(integer);
    }
    if let Some(float) = value.as_float() {
        return float_to_integer_floor(float).ok_or_else(|| {
            NativeErrorKind::TypeError {
                index: 1,
                expected: "integer",
            }
            .into()
        });
    }
    if let Some(bytes) = runtime.string_bytes(value)
        && let Some(number) = parse_standard_number(bytes)
    {
        return select_index(runtime, Some(number));
    }
    Err(NativeErrorKind::TypeError {
        index: 1,
        expected: "integer",
    }
    .into())
}

fn float_to_integer_floor(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let value = value.floor();
    if value < i64::MIN as f64 || value > i64::MAX as f64 {
        return None;
    }
    Some(value as i64)
}

#[cfg(test)]
mod tests;
