//! Executable UTF-8 library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

const MAX_UNICODE: u32 = 0x10_FFFF;
const MAX_UTF: u32 = 0x7FFF_FFFF;

/// Executable UTF-8 library functions currently implemented.
pub const UTF8_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Utf8, "len"),
    utf8_len,
)];

fn utf8_len(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?;
    let len = bytes.len();
    let len_integer = i64::try_from(len).expect("runtime string length must fit in LuaInteger");
    let start = relative_position(optional_integer_arg(args, 2, 1)?, len);
    let end = relative_position(optional_integer_arg(args, 3, -1)?, len);
    let lax = args.get(3).is_some_and(|value| is_truthy(*value));

    if !(1 <= start && start - 1 <= len_integer) {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 2 }.into());
    }
    if end > len_integer {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 3 }.into());
    }

    let mut position = usize::try_from(start - 1).expect("start position must be non-negative");
    let end = end - 1;
    let mut count = 0_i64;
    while i64::try_from(position).expect("position must fit in LuaInteger") <= end {
        let Some(next_position) = decode_utf8(bytes, position, !lax) else {
            let invalid_position =
                i64::try_from(position + 1).expect("position must fit in LuaInteger");
            return Ok(vec![Value::nil(), Value::integer(invalid_position)]);
        };
        position = next_position;
        count += 1;
    }

    Ok(vec![Value::integer(count)])
}

fn decode_utf8(bytes: &[u8], position: usize, strict: bool) -> Option<usize> {
    let mut c = u32::from(*bytes.get(position)?);
    let mut result = 0_u32;
    if c < 0x80 {
        result = c;
    } else {
        let mut count = 0_usize;
        while c & 0x40 != 0 {
            count += 1;
            if count > 5 {
                return None;
            }
            c <<= 1;
            let continuation = *bytes.get(position + count)?;
            if !is_continuation(continuation) {
                return None;
            }
            result = (result << 6) | u32::from(continuation & 0x3f);
        }
        result |= (c & 0x7f) << (count * 5);
        let minimum = match count {
            0 => u32::MAX,
            1 => 0x80,
            2 => 0x800,
            3 => 0x1_0000,
            4 => 0x20_0000,
            5 => 0x400_0000,
            _ => return None,
        };
        if result > MAX_UTF || result < minimum {
            return None;
        }
        if strict && (result > MAX_UNICODE || (0xd800..=0xdfff).contains(&result)) {
            return None;
        }
    }
    Some(position + utf8_sequence_len(result))
}

fn utf8_sequence_len(codepoint: u32) -> usize {
    match codepoint {
        0x0000..=0x007f => 1,
        0x0080..=0x07ff => 2,
        0x0800..=0xffff => 3,
        0x1_0000..=0x1f_ffff => 4,
        0x20_0000..=0x3ff_ffff => 5,
        _ => 6,
    }
}

fn is_continuation(byte: u8) -> bool {
    byte & 0xc0 == 0x80
}

fn optional_integer_arg(args: &[Value], index: usize, default: i64) -> Result<i64, NativeError> {
    match args.get(index - 1) {
        Some(value) if value.is_nil() => Ok(default),
        Some(value) => value.as_integer().ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "integer",
            }
            .into(),
        ),
        None => Ok(default),
    }
}

fn relative_position(position: i64, len: usize) -> i64 {
    if position >= 0 {
        return position;
    }
    if position.unsigned_abs() > len as u64 {
        return 0;
    }
    i64::try_from(len).expect("runtime string length must fit in LuaInteger") + position + 1
}

fn string_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<&[u8], NativeError> {
    runtime.short_string_bytes(value).ok_or(
        NativeErrorKind::TypeError {
            index,
            expected: "string",
        }
        .into(),
    )
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{UTF8_NATIVE_FUNCTIONS, utf8_len};
    use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::table_index(index)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_table_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    #[test]
    fn utf8_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = UTF8_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Utf8, "len")));
    }

    #[test]
    fn utf8_len_counts_valid_sequences() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"a\xc3\xa9\xf0\x9d\x84\x9e");

        assert_eq!(
            utf8_len(&mut runtime, &[value]).expect("utf8.len should pass"),
            vec![Value::integer(3)]
        );
        assert_eq!(
            utf8_len(&mut runtime, &[value, Value::integer(2), Value::integer(3)])
                .expect("utf8.len should pass"),
            vec![Value::integer(1)]
        );
        assert_eq!(
            utf8_len(
                &mut runtime,
                &[value, Value::integer(2), Value::integer(-1)]
            )
            .expect("utf8.len should pass"),
            vec![Value::integer(2)]
        );
    }

    #[test]
    fn utf8_len_reports_first_invalid_byte() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"a\xc0\x80b");

        assert_eq!(
            utf8_len(&mut runtime, &[value]).expect("utf8.len should pass"),
            vec![Value::nil(), Value::integer(2)]
        );
    }

    #[test]
    fn utf8_len_lax_allows_non_strict_codepoints() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"\xed\xa0\x80");

        assert_eq!(
            utf8_len(&mut runtime, &[value]).expect("utf8.len should pass"),
            vec![Value::nil(), Value::integer(1)]
        );
        assert_eq!(
            utf8_len(
                &mut runtime,
                &[
                    value,
                    Value::integer(1),
                    Value::integer(-1),
                    Value::boolean(true)
                ],
            )
            .expect("utf8.len should pass"),
            vec![Value::integer(1)]
        );
    }

    #[test]
    fn utf8_len_rejects_out_of_bounds_positions() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"abc");

        assert_eq!(
            utf8_len(&mut runtime, &[value, Value::integer(0)])
                .expect_err("zero start should fail")
                .kind(),
            &NativeErrorKind::ArgumentOutOfRange { index: 2 }
        );
        assert_eq!(
            utf8_len(&mut runtime, &[value, Value::integer(1), Value::integer(4)])
                .expect_err("large end should fail")
                .kind(),
            &NativeErrorKind::ArgumentOutOfRange { index: 3 }
        );
    }
}
