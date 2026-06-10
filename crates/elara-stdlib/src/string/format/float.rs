//! Non-hexadecimal float conversion helpers for `string.format`.

use elara_core::LuaFloat;

use crate::NativeError;

use super::{conversion_index, invalid_format_spec, parse_two_digit_field};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FloatFormatSpec {
    conversion: u8,
    width: usize,
    precision: Option<usize>,
    left_adjust: bool,
    force_sign: bool,
    alternate_form: bool,
    zero_pad: bool,
    space_sign: bool,
    pub(super) next_index: usize,
}

pub(super) fn parse_float_spec(
    format: &[u8],
    start: usize,
) -> Result<Option<FloatFormatSpec>, NativeError> {
    if !matches!(
        format.get(start),
        Some(
            b'-' | b'+' | b'#' | b'0' | b' ' | b'1'
                ..=b'9' | b'.' | b'e' | b'E' | b'f' | b'g' | b'G'
        )
    ) {
        return Ok(None);
    }

    let Some(conversion) = conversion_index(format, start) else {
        return Ok(None);
    };
    let Some(spec @ (b'e' | b'E' | b'f' | b'g' | b'G')) = format.get(conversion).copied() else {
        return Ok(None);
    };

    let mut cursor = start;
    let mut parsed = ParsedFloatFlags::default();
    while let Some(byte) = format.get(cursor).copied() {
        match byte {
            b'-' => parsed.left_adjust = true,
            b'+' => parsed.force_sign = true,
            b'#' => parsed.alternate_form = true,
            b'0' => parsed.zero_pad = true,
            b' ' => parsed.space_sign = true,
            _ => break,
        }
        cursor += 1;
    }

    let width = parse_two_digit_field(format, &mut cursor)?.unwrap_or(0);
    let precision = if format.get(cursor) == Some(&b'.') {
        cursor += 1;
        Some(parse_two_digit_field(format, &mut cursor)?.unwrap_or(0))
    } else {
        None
    };
    if cursor != conversion {
        return Err(invalid_format_spec());
    }

    Ok(Some(FloatFormatSpec {
        conversion: spec,
        width,
        precision,
        left_adjust: parsed.left_adjust,
        force_sign: parsed.force_sign,
        alternate_form: parsed.alternate_form,
        zero_pad: parsed.zero_pad,
        space_sign: parsed.space_sign,
        next_index: conversion + 1,
    }))
}

#[derive(Default)]
struct ParsedFloatFlags {
    left_adjust: bool,
    force_sign: bool,
    alternate_form: bool,
    zero_pad: bool,
    space_sign: bool,
}

pub(super) fn format_float_conversion(spec: FloatFormatSpec, value: LuaFloat) -> String {
    let mut formatted = format_float_body(spec, value);
    if !formatted.starts_with('-') {
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
    } else if pad_byte == '0' && matches!(formatted.as_bytes().first(), Some(b'-' | b'+' | b' ')) {
        let (sign, body) = formatted.split_at(1);
        format!("{sign}{padding}{body}")
    } else {
        format!("{padding}{formatted}")
    }
}

fn format_float_body(spec: FloatFormatSpec, value: LuaFloat) -> String {
    let precision = spec.precision.unwrap_or(6);
    match spec.conversion {
        b'e' => format_exponential_with_precision(value, false, precision, spec.alternate_form),
        b'E' => format_exponential_with_precision(value, true, precision, spec.alternate_form),
        b'f' => format_fixed_float(value, precision, spec.alternate_form),
        b'g' => format_general_float(value, false, precision.max(1), spec.alternate_form),
        b'G' => format_general_float(value, true, precision.max(1), spec.alternate_form),
        _ => unreachable!("caller filters float conversion specifiers"),
    }
}

fn format_fixed_float(value: LuaFloat, precision: usize, alternate_form: bool) -> String {
    let mut formatted = format!("{value:.precision$}");
    if alternate_form && value.is_finite() && !formatted.contains('.') {
        formatted.push('.');
    }
    formatted
}

fn format_exponential_with_precision(
    value: LuaFloat,
    upper: bool,
    precision: usize,
    alternate_form: bool,
) -> String {
    let formatted = if upper {
        format!("{value:.precision$E}")
    } else {
        format!("{value:.precision$e}")
    };
    let Some((mut mantissa, exponent)) = split_exponential(&formatted, upper) else {
        return if upper {
            formatted.to_ascii_uppercase()
        } else {
            formatted
        };
    };
    if alternate_form && value.is_finite() && !mantissa.contains('.') {
        mantissa.push('.');
    }
    let exponent = exponent
        .parse::<i32>()
        .expect("Rust exponential formatter emits an integer exponent");
    let marker = if upper { 'E' } else { 'e' };
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}{marker}{sign}{:02}", exponent.abs())
}

fn split_exponential(formatted: &str, upper: bool) -> Option<(String, &str)> {
    let marker = if upper { 'E' } else { 'e' };
    let (mantissa, exponent) = formatted.split_once(marker)?;
    Some((mantissa.to_owned(), exponent))
}

fn format_general_float(
    value: LuaFloat,
    upper: bool,
    precision: usize,
    alternate_form: bool,
) -> String {
    if !value.is_finite() {
        let formatted = value.to_string();
        return if upper {
            formatted.to_ascii_uppercase()
        } else {
            formatted
        };
    }
    if value == 0.0 {
        return if alternate_form {
            "0.".to_owned() + &"0".repeat(precision - 1)
        } else {
            "0".to_owned()
        };
    }

    let precision = i32::try_from(precision).expect("format precision fits in i32");
    let exponent = value.abs().log10().floor() as i32;
    if !(-4..precision).contains(&exponent) {
        let formatted = format_exponential_with_precision(
            value,
            upper,
            (precision - 1) as usize,
            alternate_form,
        );
        let marker = if upper { 'E' } else { 'e' };
        let (mantissa, exponent) = formatted
            .split_once(marker)
            .expect("exponential formatter returns an exponent marker");
        let mantissa = if alternate_form {
            mantissa.to_owned()
        } else {
            trim_float_zeros(mantissa).to_owned()
        };
        return format!("{mantissa}{marker}{exponent}");
    }

    let decimals = (precision - exponent - 1).max(0) as usize;
    let formatted = format!("{value:.decimals$}");
    if alternate_form {
        formatted
    } else {
        trim_float_zeros(&formatted).to_owned()
    }
}

fn trim_float_zeros(value: &str) -> &str {
    let trimmed = value.trim_end_matches('0');
    trimmed.strip_suffix('.').unwrap_or(trimmed)
}
