//! Integer conversion helpers for `string.format`.

use elara_core::LuaInteger;

use crate::NativeError;

use super::{conversion_index, invalid_format_spec, parse_two_digit_field};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct IntegerFormatSpec {
    pub(super) conversion: u8,
    pub(super) width: usize,
    left_adjust: bool,
    zero_pad: bool,
    force_sign: bool,
    space_sign: bool,
    alternate_form: bool,
    pub(super) next_index: usize,
}

pub(super) fn parse_integer_width_spec(
    format: &[u8],
    start: usize,
) -> Result<Option<IntegerFormatSpec>, NativeError> {
    if !matches!(
        format.get(start),
        Some(b'-' | b'+' | b'#' | b'0' | b' ' | b'1'..=b'9')
    ) {
        return Ok(None);
    }

    let Some(conversion) = conversion_index(format, start) else {
        return Ok(None);
    };
    let Some(spec @ (b'd' | b'i' | b'u' | b'o' | b'x' | b'X')) = format.get(conversion).copied()
    else {
        return Ok(None);
    };

    let mut cursor = start;
    let mut parsed = ParsedIntegerFlags::default();
    while let Some(byte) = format.get(cursor).copied() {
        match byte {
            b'-' => parsed.left_adjust = true,
            b'+' => parsed.force_sign = true,
            b'#' => parsed.alternate_form = true,
            b'0' => parsed.zero_pad = true,
            b' ' => parsed.space_sign = true,
            _ => break,
        }
        parsed.saw_flag = true;
        cursor += 1;
    }
    if (parsed.force_sign || parsed.space_sign) && !matches!(spec, b'd' | b'i') {
        return Err(invalid_format_spec());
    }
    if parsed.alternate_form && !matches!(spec, b'o' | b'x' | b'X') {
        return Err(invalid_format_spec());
    }
    let has_width = matches!(format.get(cursor), Some(b'1'..=b'9'));
    let has_flags_only = parsed.saw_flag && cursor == conversion;
    if !has_width && !has_flags_only {
        return Ok(None);
    }
    let width = parse_two_digit_field(format, &mut cursor)?.unwrap_or(0);
    if cursor != conversion {
        return Err(invalid_format_spec());
    }
    Ok(Some(IntegerFormatSpec {
        conversion: spec,
        width,
        left_adjust: parsed.left_adjust,
        zero_pad: parsed.zero_pad,
        force_sign: parsed.force_sign,
        space_sign: parsed.space_sign,
        alternate_form: parsed.alternate_form,
        next_index: conversion + 1,
    }))
}

#[derive(Default)]
struct ParsedIntegerFlags {
    left_adjust: bool,
    zero_pad: bool,
    force_sign: bool,
    space_sign: bool,
    alternate_form: bool,
    saw_flag: bool,
}

pub(super) fn format_integer_conversion(spec: u8, value: LuaInteger) -> String {
    match spec {
        b'd' | b'i' => value.to_string(),
        b'u' => (value as u64).to_string(),
        b'o' => format!("{:o}", value as u64),
        b'x' => format!("{:x}", value as u64),
        b'X' => format!("{:X}", value as u64),
        _ => unreachable!("caller filters integer conversion specifiers"),
    }
}

pub(super) fn format_integer_width_conversion(
    spec: IntegerFormatSpec,
    value: LuaInteger,
) -> String {
    let mut formatted = format_integer_conversion(spec.conversion, value);
    if spec.alternate_form {
        apply_integer_alternate_form(spec.conversion, value, &mut formatted);
    }
    if matches!(spec.conversion, b'd' | b'i') && !formatted.starts_with('-') {
        if spec.force_sign {
            formatted.insert(0, '+');
        } else if spec.space_sign {
            formatted.insert(0, ' ');
        }
    }
    if formatted.len() >= spec.width {
        return formatted;
    }

    let pad_byte = if spec.zero_pad && !spec.left_adjust {
        '0'
    } else {
        ' '
    };
    let padding = pad_byte.to_string().repeat(spec.width - formatted.len());
    if spec.left_adjust {
        format!("{formatted}{padding}")
    } else if pad_byte == '0' && zero_padding_prefix_len(spec, &formatted) > 0 {
        let prefix_len = zero_padding_prefix_len(spec, &formatted);
        let (prefix, body) = formatted.split_at(prefix_len);
        format!("{prefix}{padding}{body}")
    } else {
        format!("{padding}{formatted}")
    }
}

fn apply_integer_alternate_form(spec: u8, value: LuaInteger, formatted: &mut String) {
    match spec {
        b'o' if !formatted.starts_with('0') => formatted.insert(0, '0'),
        b'x' if value != 0 => formatted.insert_str(0, "0x"),
        b'X' if value != 0 => formatted.insert_str(0, "0X"),
        _ => {}
    }
}

fn zero_padding_prefix_len(spec: IntegerFormatSpec, formatted: &str) -> usize {
    if matches!(spec.conversion, b'd' | b'i')
        && matches!(formatted.as_bytes().first(), Some(b'-' | b'+' | b' '))
    {
        1
    } else if spec.alternate_form && matches!(spec.conversion, b'x' | b'X') && formatted.len() >= 2
    {
        2
    } else {
        0
    }
}
