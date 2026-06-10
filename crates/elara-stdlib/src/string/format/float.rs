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
                ..=b'9' | b'.' | b'a' | b'A' | b'e' | b'E' | b'f' | b'g' | b'G'
        )
    ) {
        return Ok(None);
    }

    let Some(conversion) = conversion_index(format, start) else {
        return Ok(None);
    };
    let Some(spec @ (b'a' | b'A' | b'e' | b'E' | b'f' | b'g' | b'G')) =
        format.get(conversion).copied()
    else {
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
    } else if pad_byte == '0' && zero_padding_prefix_len(spec, &formatted) > 0 {
        let prefix_len = zero_padding_prefix_len(spec, &formatted);
        let (prefix, body) = formatted.split_at(prefix_len);
        format!("{prefix}{padding}{body}")
    } else {
        format!("{padding}{formatted}")
    }
}

fn zero_padding_prefix_len(spec: FloatFormatSpec, formatted: &str) -> usize {
    let sign_len = usize::from(matches!(
        formatted.as_bytes().first(),
        Some(b'-' | b'+' | b' ')
    ));
    if matches!(spec.conversion, b'a' | b'A')
        && formatted
            .get(sign_len..)
            .is_some_and(|body| body.starts_with("0x") || body.starts_with("0X"))
    {
        sign_len + 2
    } else {
        sign_len
    }
}

fn format_hex_float_conversion(spec: FloatFormatSpec, value: LuaFloat) -> String {
    let mut formatted = format_hex_float_lower(value, spec.alternate_form, spec.precision);
    if spec.conversion == b'A' {
        formatted = formatted.to_ascii_uppercase();
    }
    formatted
}

fn format_hex_float_lower(
    value: LuaFloat,
    alternate_form: bool,
    precision: Option<usize>,
) -> String {
    if value.is_nan() || value.is_infinite() {
        return value.to_string();
    }

    let bits = value.to_bits();
    let sign = if bits >> 63 == 0 { "" } else { "-" };
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction_bits = bits & 0x000f_ffff_ffff_ffff;
    if exponent_bits == 0 && fraction_bits == 0 {
        return format_hex_zero(sign, alternate_form, precision);
    }

    let (head, exponent) = if exponent_bits == 0 {
        (0, -1022)
    } else {
        (1, exponent_bits - 1023)
    };
    let full_digits = format!("{fraction_bits:013x}");
    let (head, digits) = if let Some(precision) = precision {
        round_hex_fraction(head, &full_digits, precision)
    } else {
        (head, trimmed_hex_fraction(&full_digits))
    };
    let head = hex_digit(head);
    let needs_point = alternate_form || precision.is_some_and(|precision| precision > 0);
    if digits.is_empty() && !needs_point {
        format!("{sign}0x{head}p{exponent:+}")
    } else if digits.is_empty() {
        format!("{sign}0x{head}.p{exponent:+}")
    } else {
        format!("{sign}0x{head}.{digits}p{exponent:+}")
    }
}

fn format_hex_zero(sign: &str, alternate_form: bool, precision: Option<usize>) -> String {
    match precision {
        Some(0) if alternate_form => format!("{sign}0x0.p+0"),
        Some(0) => format!("{sign}0x0p+0"),
        Some(precision) => format!("{sign}0x0.{}p+0", "0".repeat(precision)),
        None if alternate_form => format!("{sign}0x0.p+0"),
        None => format!("{sign}0x0p+0"),
    }
}

fn round_hex_fraction(head: u8, full_digits: &str, precision: usize) -> (u8, String) {
    let mut retained = full_digits
        .bytes()
        .take(precision)
        .map(hex_digit_value)
        .collect::<Vec<_>>();
    if precision >= full_digits.len() {
        retained.resize(precision, 0);
        return (head, retained.into_iter().map(hex_digit).collect());
    }

    let next = hex_digit_value(full_digits.as_bytes()[precision]);
    let rest_nonzero = full_digits.as_bytes()[precision + 1..]
        .iter()
        .copied()
        .map(hex_digit_value)
        .any(|digit| digit != 0);
    let last_retained_odd = !retained.last().copied().unwrap_or(head).is_multiple_of(2);
    let round_up = next > 8 || next == 8 && (rest_nonzero || last_retained_odd);
    if !round_up {
        return (head, retained.into_iter().map(hex_digit).collect());
    }

    if retained.is_empty() {
        return (head + 1, String::new());
    }
    for digit in retained.iter_mut().rev() {
        if *digit < 15 {
            *digit += 1;
            return (head, retained.into_iter().map(hex_digit).collect());
        }
        *digit = 0;
    }
    (head + 1, retained.into_iter().map(hex_digit).collect())
}

fn trimmed_hex_fraction(full_digits: &str) -> String {
    full_digits.trim_end_matches('0').to_owned()
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + value - 10),
        _ => unreachable!("hex digit fits in one nibble"),
    }
}

fn hex_digit_value(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => unreachable!("formatted hex fraction contains only hex digits"),
    }
}

fn format_float_body(spec: FloatFormatSpec, value: LuaFloat) -> String {
    let precision = spec.precision.unwrap_or(6);
    match spec.conversion {
        b'a' | b'A' => format_hex_float_conversion(spec, value),
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
