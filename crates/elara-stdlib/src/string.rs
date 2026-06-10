//! Executable string-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable string-library functions currently implemented.
pub const STRING_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "byte"), string_byte),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "len"), string_len),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "lower"), string_lower),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "rep"), string_rep),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "reverse"), string_reverse),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "sub"), string_sub),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "upper"), string_upper),
];

fn string_len(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?;
    let len = i64::try_from(bytes.len()).expect("runtime string length must fit in LuaInteger");
    Ok(vec![Value::integer(len)])
}

fn string_byte(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?;
    let len = bytes.len();
    let start_arg = optional_integer_arg(args, 2, 1)?;
    let start = relative_start(start_arg, len);
    let end = relative_end(optional_integer_arg(args, 3, start_arg)?, len);

    if start > end {
        return Ok(Vec::new());
    }

    let count = (end - start) + 1;
    if count > i32::MAX as usize {
        return Err(string_slice_too_long());
    }

    Ok(bytes[(start - 1)..end]
        .iter()
        .map(|byte| Value::integer(i64::from(*byte)))
        .collect())
}

fn string_lower(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?;
    let lowered: Vec<_> = bytes.iter().map(u8::to_ascii_lowercase).collect();
    Ok(vec![runtime.intern_short_string(&lowered)?])
}

fn string_upper(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?;
    let uppered: Vec<_> = bytes.iter().map(u8::to_ascii_uppercase).collect();
    Ok(vec![runtime.intern_short_string(&uppered)?])
}

fn string_reverse(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?;
    let reversed: Vec<_> = bytes.iter().rev().copied().collect();
    Ok(vec![runtime.intern_short_string(&reversed)?])
}

fn string_rep(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?.to_vec();
    let count = args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?
        .as_integer()
        .ok_or(NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        })?;
    let separator = match args.get(2) {
        Some(value) => string_arg(runtime, *value, 3)?.to_vec(),
        None => Vec::new(),
    };

    if count <= 0 {
        return Ok(vec![runtime.intern_short_string(b"")?]);
    }

    let count = usize::try_from(count).map_err(|_| string_result_too_large())?;
    let repeated_len = bytes
        .len()
        .checked_add(separator.len())
        .ok_or_else(string_result_too_large)?;
    let total_len = count
        .checked_mul(repeated_len)
        .and_then(|len| len.checked_sub(separator.len()))
        .ok_or_else(string_result_too_large)?;

    let mut output = Vec::with_capacity(total_len);
    for index in 0..count {
        if index > 0 {
            output.extend_from_slice(&separator);
        }
        output.extend_from_slice(&bytes);
    }
    Ok(vec![runtime.intern_short_string(&output)?])
}

fn string_sub(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?.to_vec();
    let len = bytes.len();
    let start = relative_start(integer_arg(args, 2)?, len);
    let end = relative_end(optional_integer_arg(args, 3, -1)?, len);

    if start > end {
        return Ok(vec![runtime.intern_short_string(b"")?]);
    }

    let slice = &bytes[(start - 1)..end];
    Ok(vec![runtime.intern_short_string(slice)?])
}

fn integer_arg(args: &[Value], index: usize) -> Result<i64, NativeError> {
    args.get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?
        .as_integer()
        .ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "integer",
            }
            .into(),
        )
}

fn optional_integer_arg(args: &[Value], index: usize, default: i64) -> Result<i64, NativeError> {
    match args.get(index - 1) {
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

fn relative_start(position: i64, len: usize) -> usize {
    let len = i64::try_from(len).expect("runtime string length must fit in LuaInteger");
    if position > 0 {
        usize::try_from(position).unwrap_or(usize::MAX)
    } else if position == 0 || position < -len {
        1
    } else {
        usize::try_from(len + position + 1).expect("relative start is positive")
    }
}

fn relative_end(position: i64, len: usize) -> usize {
    let len_integer = i64::try_from(len).expect("runtime string length must fit in LuaInteger");
    if position > len_integer {
        len
    } else if position >= 0 {
        usize::try_from(position).expect("relative end is non-negative")
    } else if position < -len_integer {
        0
    } else {
        usize::try_from(len_integer + position + 1).expect("relative end is non-negative")
    }
}

fn string_result_too_large() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "resulting string too large".into(),
    }
    .into()
}

fn string_slice_too_long() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "string slice too long".into(),
    }
    .into()
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

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{
        STRING_NATIVE_FUNCTIONS, string_byte, string_len, string_lower, string_rep, string_reverse,
        string_sub, string_upper,
    };
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
    fn string_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = STRING_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "byte")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "len")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "lower")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "rep")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "reverse")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "sub")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "upper")));
    }

    #[test]
    fn string_len_returns_byte_length() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"a\0bc");

        assert_eq!(
            string_len(&mut runtime, &[value]).expect("string.len should pass"),
            vec![Value::integer(4)]
        );
    }

    #[test]
    fn string_len_reports_type_errors() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            string_len(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string"
            }
        );
    }

    #[test]
    fn string_byte_returns_integer_bytes_for_range() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"A\0C");

        assert_eq!(
            string_byte(&mut runtime, &[value, Value::integer(1), Value::integer(3)])
                .expect("byte should pass"),
            vec![Value::integer(65), Value::integer(0), Value::integer(67)]
        );
    }

    #[test]
    fn string_byte_defaults_end_to_start() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"AZ");

        assert_eq!(
            string_byte(&mut runtime, &[value, Value::integer(2)]).expect("byte should pass"),
            vec![Value::integer(90)]
        );
    }

    #[test]
    fn string_byte_empty_interval_returns_no_values() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"AZ");

        assert_eq!(
            string_byte(&mut runtime, &[value, Value::integer(3), Value::integer(2)])
                .expect("byte should pass"),
            Vec::<Value>::new()
        );
    }

    #[test]
    fn string_case_mapping_returns_transformed_bytes() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"AbC123");

        let lowered = string_lower(&mut runtime, &[value]).expect("lower should pass");
        let uppered = string_upper(&mut runtime, &[value]).expect("upper should pass");

        assert_eq!(
            runtime.short_string_bytes(lowered[0]),
            Some(b"abc123".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(uppered[0]),
            Some(b"ABC123".as_slice())
        );
    }

    #[test]
    fn string_reverse_returns_reversed_bytes() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"a\0bc");
        let reversed = string_reverse(&mut runtime, &[value]).expect("reverse should pass");

        assert_eq!(
            runtime.short_string_bytes(reversed[0]),
            Some(b"cb\0a".as_slice())
        );
    }

    #[test]
    fn string_rep_repeats_with_optional_separator() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"ab");
        let separator = runtime.push_string(b",");
        let repeated = string_rep(&mut runtime, &[value, Value::integer(3), separator])
            .expect("rep should pass");

        assert_eq!(
            runtime.short_string_bytes(repeated[0]),
            Some(b"ab,ab,ab".as_slice())
        );
    }

    #[test]
    fn string_rep_non_positive_count_returns_empty_string() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"ab");
        let repeated =
            string_rep(&mut runtime, &[value, Value::integer(0)]).expect("rep should pass");

        assert_eq!(
            runtime.short_string_bytes(repeated[0]),
            Some(b"".as_slice())
        );
    }

    #[test]
    fn string_rep_requires_integer_count() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"ab");

        assert_eq!(
            string_rep(&mut runtime, &[value, Value::float(2.5)])
                .expect_err("float count should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "integer"
            }
        );
    }

    #[test]
    fn string_sub_uses_one_based_and_negative_positions() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"abcdef");
        let sliced = string_sub(
            &mut runtime,
            &[value, Value::integer(2), Value::integer(-2)],
        )
        .expect("sub should pass");

        assert_eq!(
            runtime.short_string_bytes(sliced[0]),
            Some(b"bcde".as_slice())
        );
    }

    #[test]
    fn string_sub_defaults_end_to_last_byte() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"abcdef");
        let sliced =
            string_sub(&mut runtime, &[value, Value::integer(-3)]).expect("sub should pass");

        assert_eq!(
            runtime.short_string_bytes(sliced[0]),
            Some(b"def".as_slice())
        );
    }

    #[test]
    fn string_sub_returns_empty_when_start_exceeds_end() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"abcdef");
        let sliced = string_sub(&mut runtime, &[value, Value::integer(5), Value::integer(3)])
            .expect("sub should pass");

        assert_eq!(runtime.short_string_bytes(sliced[0]), Some(b"".as_slice()));
    }
}
