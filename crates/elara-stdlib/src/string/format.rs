//! `string.format` native implementation.

use elara_core::{LuaFloat, LuaInteger, Value};

use crate::{NativeError, NativeErrorKind, NativeRuntime, number::parse_standard_number};

use super::string_arg;

mod float;
mod integer;
use float::{format_float_conversion, format_hex_float_literal, parse_float_spec};
use integer::{
    format_integer_conversion, format_integer_width_conversion, parse_integer_width_spec,
};

pub(super) fn string_format(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let format = string_arg(
        runtime,
        *args
            .first()
            .ok_or(NativeErrorKind::MissingArgument { index: 1 })?,
        1,
    )?
    .to_vec();
    let mut output = Vec::with_capacity(format.len());
    let mut index = 0;
    let mut arg_index = 1;
    while index < format.len() {
        if format[index] != b'%' {
            output.push(format[index]);
            index += 1;
        } else if format.get(index + 1) == Some(&b'%') {
            output.push(b'%');
            index += 2;
        } else if let Some((spec, next_index)) = parse_string_spec(&format, index + 1)? {
            let value = next_format_arg(args, arg_index)?;
            output.extend_from_slice(&format_string_arg(runtime, value, &spec)?);
            arg_index += 1;
            index = next_index;
        } else if format.get(index + 1) == Some(&b'q') {
            let value = next_format_arg(args, arg_index)?;
            output.extend_from_slice(&format_quoted_arg(runtime, value, arg_index + 1)?);
            arg_index += 1;
            index += 2;
        } else if has_modified_quote_spec(&format, index + 1) {
            return Err(NativeErrorKind::RuntimeError {
                message: "specifier '%q' cannot have modifiers".into(),
            }
            .into());
        } else if format.get(index + 1) == Some(&b'p') {
            let value = next_format_arg(args, arg_index)?;
            output.extend_from_slice(format_pointer_arg(runtime, value).as_bytes());
            arg_index += 1;
            index += 2;
        } else if let Some((spec, conversion, next_index)) =
            parse_character_pointer_spec(&format, index + 1)?
        {
            let value = next_format_arg(args, arg_index)?;
            let bytes = if conversion == b'c' {
                vec![integer_format_arg(runtime, value, arg_index + 1)? as u8]
            } else {
                format_pointer_arg(runtime, value).into_bytes()
            };
            output.extend_from_slice(&format_padded_bytes(bytes, &spec));
            arg_index += 1;
            index = next_index;
        } else if let Some(spec) = parse_float_spec(&format, index + 1)? {
            let value =
                float_format_arg(runtime, next_format_arg(args, arg_index)?, arg_index + 1)?;
            output.extend_from_slice(format_float_conversion(spec, value).as_bytes());
            arg_index += 1;
            index = spec.next_index;
        } else if let Some(spec) = parse_integer_width_spec(&format, index + 1)? {
            let value =
                integer_format_arg(runtime, next_format_arg(args, arg_index)?, arg_index + 1)?;
            output.extend_from_slice(format_integer_width_conversion(spec, value).as_bytes());
            arg_index += 1;
            index = spec.next_index;
        } else if let Some(spec @ (b'c' | b'd' | b'i' | b'u' | b'o' | b'x' | b'X')) =
            format.get(index + 1).copied()
        {
            let value =
                integer_format_arg(runtime, next_format_arg(args, arg_index)?, arg_index + 1)?;
            if spec == b'c' {
                output.push(value as u8);
            } else {
                output.extend_from_slice(format_integer_conversion(spec, value).as_bytes());
            }
            arg_index += 1;
            index += 2;
        } else {
            return Err(NativeErrorKind::RuntimeError {
                message: "string.format conversions are not supported yet".into(),
            }
            .into());
        }
    }

    Ok(vec![runtime.intern_string(&output)?])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StringFormatSpec {
    left_adjust: bool,
    width: Option<usize>,
    precision: Option<usize>,
}

impl StringFormatSpec {
    const PLAIN: Self = Self {
        left_adjust: false,
        width: None,
        precision: None,
    };

    const fn has_modifiers(self) -> bool {
        self.left_adjust || self.width.is_some() || self.precision.is_some()
    }
}

fn next_format_arg(args: &[Value], arg_index: usize) -> Result<Value, NativeError> {
    args.get(arg_index).copied().ok_or(
        NativeErrorKind::MissingArgument {
            index: arg_index + 1,
        }
        .into(),
    )
}

fn parse_string_spec(
    format: &[u8],
    start: usize,
) -> Result<Option<(StringFormatSpec, usize)>, NativeError> {
    if format.get(start) == Some(&b's') {
        return Ok(Some((StringFormatSpec::PLAIN, start + 1)));
    }

    let Some(conversion) = conversion_index(format, start) else {
        return Ok(None);
    };
    if format.get(conversion) != Some(&b's') {
        return Ok(None);
    }

    let mut cursor = start;
    let mut left_adjust = false;
    while format.get(cursor) == Some(&b'-') {
        left_adjust = true;
        cursor += 1;
    }
    if matches!(format.get(cursor), Some(b'+' | b'#' | b'0' | b' ')) {
        return Err(invalid_format_spec());
    }

    let width = parse_two_digit_field(format, &mut cursor)?;
    let precision = if format.get(cursor) == Some(&b'.') {
        cursor += 1;
        Some(parse_two_digit_field(format, &mut cursor)?.unwrap_or(0))
    } else {
        None
    };
    if cursor != conversion {
        return Err(invalid_format_spec());
    }

    Ok(Some((
        StringFormatSpec {
            left_adjust,
            width,
            precision,
        },
        conversion + 1,
    )))
}

fn parse_character_pointer_spec(
    format: &[u8],
    start: usize,
) -> Result<Option<(StringFormatSpec, u8, usize)>, NativeError> {
    let Some(conversion) = conversion_index(format, start) else {
        return Ok(None);
    };
    let Some(spec @ (b'c' | b'p')) = format.get(conversion).copied() else {
        return Ok(None);
    };

    let mut cursor = start;
    let mut left_adjust = false;
    while format.get(cursor) == Some(&b'-') {
        left_adjust = true;
        cursor += 1;
    }
    if matches!(format.get(cursor), Some(b'+' | b'#' | b'0' | b' ' | b'.')) {
        return Err(invalid_format_spec());
    }

    let width = parse_two_digit_field(format, &mut cursor)?;
    if cursor != conversion {
        return Err(invalid_format_spec());
    }

    Ok(Some((
        StringFormatSpec {
            left_adjust,
            width,
            precision: None,
        },
        spec,
        conversion + 1,
    )))
}

fn conversion_index(format: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;
    while let Some(byte) = format.get(cursor) {
        if !matches!(byte, b'-' | b'+' | b'#' | b'0' | b' ' | b'1'..=b'9' | b'.') {
            return byte.is_ascii_alphabetic().then_some(cursor);
        }
        cursor += 1;
    }
    None
}

fn has_modified_quote_spec(format: &[u8], start: usize) -> bool {
    conversion_index(format, start)
        .is_some_and(|conversion| conversion != start && format.get(conversion) == Some(&b'q'))
}

fn parse_two_digit_field(format: &[u8], cursor: &mut usize) -> Result<Option<usize>, NativeError> {
    let start = *cursor;
    while format.get(*cursor).is_some_and(u8::is_ascii_digit) {
        *cursor += 1;
    }
    let digits = *cursor - start;
    if digits > 2 {
        return Err(invalid_format_spec());
    }
    if digits == 0 {
        return Ok(None);
    }
    let text = core::str::from_utf8(&format[start..*cursor]).expect("ASCII digits are valid UTF-8");
    Ok(Some(
        text.parse::<usize>()
            .expect("two ASCII digits fit in usize"),
    ))
}

fn invalid_format_spec() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "invalid conversion specification".into(),
    }
    .into()
}

fn format_string_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    spec: &StringFormatSpec,
) -> Result<Vec<u8>, NativeError> {
    let mut bytes = runtime
        .string_bytes(value)
        .map_or_else(|| tostring_bytes(value).into_bytes(), <[u8]>::to_vec);
    if !spec.has_modifiers() {
        return Ok(bytes);
    }
    if bytes.contains(&0) {
        return Err(NativeErrorKind::RuntimeError {
            message: "string contains zeros".into(),
        }
        .into());
    }
    if let Some(precision) = spec.precision {
        bytes.truncate(precision);
    }

    let width = spec.width.unwrap_or(0);
    if bytes.len() >= width {
        return Ok(bytes);
    }
    let padding = vec![b' '; width - bytes.len()];
    if spec.left_adjust {
        bytes.extend_from_slice(&padding);
        Ok(bytes)
    } else {
        let mut output = padding;
        output.extend_from_slice(&bytes);
        Ok(output)
    }
}

fn format_padded_bytes(mut bytes: Vec<u8>, spec: &StringFormatSpec) -> Vec<u8> {
    let width = spec.width.unwrap_or(0);
    if bytes.len() >= width {
        return bytes;
    }
    let padding = vec![b' '; width - bytes.len()];
    if spec.left_adjust {
        bytes.extend_from_slice(&padding);
        bytes
    } else {
        let mut output = padding;
        output.extend_from_slice(&bytes);
        output
    }
}

fn format_quoted_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<Vec<u8>, NativeError> {
    if let Some(bytes) = runtime.string_bytes(value) {
        return Ok(quote_string(bytes));
    }
    if value.is_nil() || value.as_bool().is_some() {
        return Ok(tostring_bytes(value).into_bytes());
    }
    if let Some(integer) = value.as_integer() {
        if integer == LuaInteger::MIN {
            return Ok(format!("0x{:x}", integer as u64).into_bytes());
        }
        return Ok(integer.to_string().into_bytes());
    }
    if let Some(float) = value.as_float() {
        return Ok(format_float_literal(float).into_bytes());
    }
    Err(NativeErrorKind::TypeError {
        index,
        expected: "literal",
    }
    .into())
}

fn format_pointer_arg(runtime: &dyn NativeRuntime, value: Value) -> String {
    if value.is_nil() || value.as_bool().is_some() || value.is_number() {
        return "(null)".to_owned();
    }
    if let Some(index) = value.as_table_index() {
        return pseudo_pointer(index);
    }
    if let Some(index) = value.as_closure_index() {
        return pseudo_pointer(index);
    }
    if let Some(index) = value.as_native_function_index() {
        return pseudo_pointer(index);
    }
    if runtime.string_bytes(value).is_some()
        || value.as_short_string().is_some()
        || value.as_long_string().is_some()
    {
        return "0x1".to_owned();
    }
    "(null)".to_owned()
}

fn pseudo_pointer(index: u32) -> String {
    format!("0x{:x}", u64::from(index) + 1)
}

fn quote_string(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len() + 2);
    output.push(b'"');
    for (index, byte) in bytes.iter().copied().enumerate() {
        if matches!(byte, b'"' | b'\\' | b'\n') {
            output.push(b'\\');
            output.push(byte);
        } else if byte.is_ascii_control() {
            output.push(b'\\');
            let next_is_digit = bytes.get(index + 1).is_some_and(u8::is_ascii_digit);
            if next_is_digit {
                output.extend_from_slice(format!("{byte:03}").as_bytes());
            } else {
                output.extend_from_slice(byte.to_string().as_bytes());
            }
        } else {
            output.push(byte);
        }
    }
    output.push(b'"');
    output
}

fn format_float_literal(value: LuaFloat) -> String {
    if value == LuaFloat::INFINITY {
        "1e9999".to_owned()
    } else if value == LuaFloat::NEG_INFINITY {
        "-1e9999".to_owned()
    } else if value.is_nan() {
        "(0/0)".to_owned()
    } else {
        format_hex_float_literal(value)
    }
}

fn integer_format_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<LuaInteger, NativeError> {
    if let Some(integer) = value.as_integer() {
        return Ok(integer);
    }
    if let Some(float) = value.as_float() {
        return floor_to_integer(float).ok_or_else(|| integer_type_error(index));
    }
    if let Some(bytes) = runtime.string_bytes(value)
        && let Some(number) = parse_standard_number(bytes)
    {
        return integer_format_arg(runtime, number, index);
    }
    Err(integer_type_error(index))
}

fn float_format_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<LuaFloat, NativeError> {
    if let Some(float) = value.to_float() {
        return Ok(float);
    }
    if let Some(bytes) = runtime.string_bytes(value)
        && let Some(number) = parse_standard_number(bytes)
    {
        return float_format_arg(runtime, number, index);
    }
    Err(NativeErrorKind::TypeError {
        index,
        expected: "number",
    }
    .into())
}

fn floor_to_integer(value: LuaFloat) -> Option<LuaInteger> {
    if !value.is_finite() {
        return None;
    }
    let value = value.floor();
    if value < LuaInteger::MIN as LuaFloat || value >= (LuaInteger::MAX as LuaFloat + 1.0) {
        return None;
    }
    Some(value as LuaInteger)
}

fn integer_type_error(index: usize) -> NativeError {
    NativeErrorKind::TypeError {
        index,
        expected: "integer",
    }
    .into()
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
        "unknown: 0x0".to_owned()
    }
}

#[cfg(test)]
mod float_tests;
#[cfg(test)]
mod tests;
