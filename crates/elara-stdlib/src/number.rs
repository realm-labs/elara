//! Numeric conversion helpers shared by standard-library natives.

use elara_core::Value;

pub(crate) fn parse_standard_number(bytes: &[u8]) -> Option<Value> {
    let text = std::str::from_utf8(trim_ascii_spaces(bytes)).ok()?;
    if text.is_empty() {
        return None;
    }
    let (negative, unsigned) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    if let Some(hex) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        return parse_hex_number(hex, negative);
    }
    text.parse::<i64>()
        .map(Value::integer)
        .or_else(|_| text.parse::<f64>().map(Value::float))
        .ok()
}

fn parse_hex_number(hex: &str, negative: bool) -> Option<Value> {
    if !hex.as_bytes().contains(&b'.')
        && !hex.as_bytes().contains(&b'p')
        && !hex.as_bytes().contains(&b'P')
    {
        return i64::from_str_radix(hex, 16)
            .ok()
            .map(|value| Value::integer(if negative { -value } else { value }));
    }

    let bytes = hex.as_bytes();
    let mut index = 0;
    let mut value = 0.0;
    let mut saw_digit = false;
    while let Some(byte) = bytes.get(index).copied() {
        if matches!(byte, b'.' | b'p' | b'P') {
            break;
        }
        let Some(digit) = ascii_digit_value(byte) else {
            break;
        };
        if digit >= 16 {
            return None;
        }
        value = value * 16.0 + f64::from(digit);
        saw_digit = true;
        index += 1;
    }

    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let mut place = 1.0 / 16.0;
        while let Some(byte) = bytes.get(index).copied() {
            if matches!(byte, b'p' | b'P') {
                break;
            }
            let Some(digit) = ascii_digit_value(byte) else {
                break;
            };
            if digit >= 16 {
                return None;
            }
            value += f64::from(digit) * place;
            place /= 16.0;
            saw_digit = true;
            index += 1;
        }
    }
    if !saw_digit {
        return None;
    }

    if matches!(bytes.get(index), Some(b'p' | b'P')) {
        index += 1;
        let exponent_start = index;
        if matches!(bytes.get(index), Some(b'-' | b'+')) {
            index += 1;
        };
        let digits_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == digits_start {
            return None;
        }
        let exponent_text =
            core::str::from_utf8(&bytes[exponent_start..index]).expect("ASCII exponent is UTF-8");
        let exponent = exponent_text.parse::<i32>().ok()?;
        value *= 2.0_f64.powi(exponent);
    }
    if index != bytes.len() {
        return None;
    }

    Some(Value::float(if negative { -value } else { value }))
}

pub(crate) fn parse_base_integer(bytes: &[u8], base: u32) -> Option<i64> {
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
