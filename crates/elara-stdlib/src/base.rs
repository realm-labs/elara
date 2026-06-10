//! Executable base-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable base-library functions currently implemented.
pub const BASE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "assert"), base_assert),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "error"), base_error),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawequal"), base_rawequal),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawget"), base_rawget),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawlen"), base_rawlen),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawset"), base_rawset),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "select"), base_select),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "tonumber"), base_tonumber),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "tostring"), base_tostring),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "type"), base_type),
];

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

fn base_select(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
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

fn error_message(runtime: &dyn NativeRuntime, value: Value) -> String {
    runtime.short_string_bytes(value).map_or_else(
        || tostring_bytes(value),
        |bytes| String::from_utf8_lossy(bytes).into_owned(),
    )
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

fn parse_standard_number(bytes: &[u8]) -> Option<Value> {
    let text = std::str::from_utf8(trim_ascii_spaces(bytes)).ok()?;
    if text.is_empty() {
        return None;
    }
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok().map(Value::integer);
    }
    text.parse::<i64>()
        .map(Value::integer)
        .or_else(|_| text.parse::<f64>().map(Value::float))
        .ok()
}

fn parse_base_integer(bytes: &[u8], base: u32) -> Option<i64> {
    let mut index = skip_ascii_spaces(bytes, 0);
    let negative = match bytes.get(index) {
        Some(b'-') => {
            index += 1;
            true
        }
        Some(b'+') => {
            index += 1;
            false
        }
        _ => false,
    };

    let mut value = 0_u64;
    let mut saw_digit = false;
    while let Some(byte) = bytes.get(index).copied().filter(u8::is_ascii_alphanumeric) {
        let digit = ascii_digit_value(byte)?;
        if digit >= base {
            return None;
        }
        value = value.wrapping_mul(u64::from(base));
        value = value.wrapping_add(u64::from(digit));
        index += 1;
        saw_digit = true;
    }
    if !saw_digit {
        return None;
    }

    index = skip_ascii_spaces(bytes, index);
    if index != bytes.len() {
        return None;
    }

    Some(if negative {
        0_u64.wrapping_sub(value) as i64
    } else {
        value as i64
    })
}

fn ascii_digit_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'Z' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn trim_ascii_spaces(bytes: &[u8]) -> &[u8] {
    let start = skip_ascii_spaces(bytes, 0);
    let end = bytes
        .iter()
        .rposition(|byte| !is_lua_space(*byte))
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

fn skip_ascii_spaces(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(|byte| is_lua_space(*byte)) {
        index += 1;
    }
    index
}

fn is_lua_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\x0c' | b'\n' | b'\r' | b'\t' | b'\x0b')
}

#[cfg(test)]
mod tests;
