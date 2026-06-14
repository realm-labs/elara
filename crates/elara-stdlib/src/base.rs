//! Executable base-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
    number::{parse_base_integer, parse_standard_number},
};

/// Executable base-library functions currently implemented.
pub const BASE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "assert"), base_assert),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "error"), base_error),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Base, "getmetatable"),
        base_getmetatable,
    ),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "ipairs"), base_ipairs),
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
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "xpcall"), base_xpcall),
];

/// Hidden helper used by `ipairs`.
pub const BASE_IPAIRS_AUX_NATIVE: NativeFunctionSpec = NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Base, "__ipairs_aux"),
    base_ipairs_aux,
);

/// Raw table traversal helper used by `next` and `pairs`.
pub const BASE_NEXT_NATIVE: NativeFunctionSpec =
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "next"), base_next);

fn base_assert(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let condition = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if is_truthy(condition) {
        Ok(args.to_vec())
    } else {
        Err(NativeErrorKind::LuaError.into())
    }
}

fn base_error(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    if let Some(level) = args.get(1).filter(|value| !value.is_nil()) {
        level.as_integer().ok_or(NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        })?;
    }
    let value = args.first().copied().unwrap_or_else(Value::nil);
    Err(NativeError::lua_error(error_message(runtime, value)))
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
    Ok(vec![
        runtime.native_function(StdLib::Base, "next")?,
        value,
        Value::nil(),
        Value::nil(),
    ])
}

fn base_pcall(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let function = args.first().copied().unwrap_or_else(Value::nil);
    let call_args = args.get(1..).unwrap_or_default();
    match runtime.protected_call(function, call_args)? {
        Ok(values) => {
            let mut results = Vec::with_capacity(values.len() + 1);
            results.push(Value::boolean(true));
            results.extend(values);
            Ok(results)
        }
        Err(message) => Ok(vec![
            Value::boolean(false),
            runtime.intern_short_string(message.as_bytes())?,
        ]),
    }
}

fn base_xpcall(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let function = args.first().copied().unwrap_or_else(Value::nil);
    let handler = args.get(1).copied().unwrap_or_else(Value::nil);
    let call_args = args.get(2..).unwrap_or_default();
    match runtime.protected_call(function, call_args)? {
        Ok(values) => {
            let mut results = Vec::with_capacity(values.len() + 1);
            results.push(Value::boolean(true));
            results.extend(values);
            Ok(results)
        }
        Err(message) => {
            let message = runtime.intern_short_string(message.as_bytes())?;
            match runtime.protected_call(handler, &[message])? {
                Ok(values) => {
                    let mut results = Vec::with_capacity(values.len() + 1);
                    results.push(Value::boolean(false));
                    results.extend(values);
                    Ok(results)
                }
                Err(handler_message) => Ok(vec![
                    Value::boolean(false),
                    runtime.intern_short_string(handler_message.as_bytes())?,
                ]),
            }
        }
    }
}

fn base_print(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    for (index, value) in args.iter().copied().enumerate() {
        if index > 0 {
            runtime.write_output(b"\t")?;
        }
        let bytes = printable_bytes(runtime, value);
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
    if let Some(bytes) = runtime.short_string_bytes(value) {
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
        .and_then(|value| runtime.short_string_bytes(*value))
        == Some(b"#")
    {
        let count = i64::try_from(args.len().saturating_sub(1)).map_err(|_| {
            NativeErrorKind::RuntimeError {
                message: "too many arguments to select".into(),
            }
        })?;
        return Ok(vec![Value::integer(count)]);
    }

    let index =
        args.first()
            .and_then(|value| value.as_integer())
            .ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "integer",
            })?;
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
                .short_string_bytes(value)
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
        .short_string_bytes(value)
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
    if runtime.short_string_bytes(value).is_some() {
        return Ok(vec![value]);
    }
    let text = tostring_bytes(value);
    Ok(vec![runtime.intern_short_string(text.as_bytes())?])
}

fn base_type(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    Ok(vec![
        runtime.intern_short_string(type_name(value).as_bytes())?,
    ])
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
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
    } else {
        format!("{}: 0x0", type_name(value))
    }
}

fn printable_bytes(runtime: &dyn NativeRuntime, value: Value) -> Vec<u8> {
    runtime
        .short_string_bytes(value)
        .map_or_else(|| tostring_bytes(value).into_bytes(), <[u8]>::to_vec)
}

fn error_message(runtime: &dyn NativeRuntime, value: Value) -> String {
    runtime.short_string_bytes(value).map_or_else(
        || tostring_bytes(value),
        |bytes| String::from_utf8_lossy(bytes).into_owned(),
    )
}

fn metatable_field_key(runtime: &mut dyn NativeRuntime) -> Result<Value, NativeError> {
    runtime.intern_short_string(b"__metatable")
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

#[cfg(test)]
mod tests;
