//! Executable string-library natives.

use std::borrow::Cow;

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
    number::parse_standard_number,
};

mod find;
mod format;
mod gmatch;
mod gsub;
mod match_;
mod pack;
mod pattern;

use find::string_find;
use format::string_format;
use gmatch::string_gmatch;
use gsub::string_gsub;
use match_::string_match;
use pack::{string_pack, string_packsize, string_unpack};

/// Executable string-library functions currently implemented.
pub const STRING_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "byte"), string_byte),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "char"), string_char),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "dump"), string_dump),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "find"), string_find),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "format"), string_format),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "gmatch"), string_gmatch),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "gsub"), string_gsub),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "len"), string_len),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "lower"), string_lower),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "match"), string_match),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "pack"), string_pack),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::String, "packsize"),
        string_packsize,
    ),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "rep"), string_rep),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "reverse"), string_reverse),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "sub"), string_sub),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "unpack"), string_unpack),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "upper"), string_upper),
];

/// Hidden helper used by `string.gmatch`.
pub const STRING_GMATCH_AUX_NATIVE: NativeFunctionSpec = NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::String, "__gmatch_aux"),
    gmatch::string_gmatch_aux,
);

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
    let start_arg = optional_integer_arg(runtime, args, 2, 1)?;
    let start = relative_start(start_arg, len);
    let end = relative_end(optional_integer_arg(runtime, args, 3, start_arg)?, len);

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

fn string_char(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let mut bytes = Vec::with_capacity(args.len());
    for index in 1..=args.len() {
        let value = integer_arg(runtime, args, index)?;
        let byte =
            u8::try_from(value).map_err(|_| NativeErrorKind::ArgumentOutOfRange { index })?;
        bytes.push(byte);
    }
    Ok(vec![runtime.intern_string(&bytes)?])
}

fn string_dump(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let function = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if function.as_closure_index().is_none() {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "Lua function",
        }
        .into());
    }
    let strip_debug = args
        .get(1)
        .is_some_and(|value| !value.is_nil() && value.as_bool() != Some(false));
    let bytes =
        runtime
            .dump_lua_function(function, strip_debug)?
            .ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "Lua function",
            })?;
    Ok(vec![runtime.intern_string(&bytes)?])
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
    Ok(vec![runtime.intern_string(&lowered)?])
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
    Ok(vec![runtime.intern_string(&uppered)?])
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
    Ok(vec![runtime.intern_string(&reversed)?])
}

fn string_rep(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?.to_vec();
    let count = integer_arg(runtime, args, 2)?;
    let separator = match args.get(2) {
        Some(value) if value.is_nil() => Vec::new(),
        Some(value) => string_arg(runtime, *value, 3)?.to_vec(),
        None => Vec::new(),
    };

    if count <= 0 {
        return Ok(vec![runtime.intern_string(b"")?]);
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
    Ok(vec![runtime.intern_string(&output)?])
}

fn string_sub(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = string_arg(runtime, value, 1)?.to_vec();
    let len = bytes.len();
    let start = relative_start(integer_arg(runtime, args, 2)?, len);
    let end = relative_end(optional_integer_arg(runtime, args, 3, -1)?, len);

    if start > end {
        return Ok(vec![runtime.intern_string(b"")?]);
    }

    let slice = &bytes[(start - 1)..end];
    Ok(vec![runtime.intern_string(slice)?])
}

fn integer_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<i64, NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    integer_value_arg(runtime, value, index)
}

fn optional_integer_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
    default: i64,
) -> Result<i64, NativeError> {
    match args.get(index - 1) {
        Some(value) if value.is_nil() => Ok(default),
        Some(value) => integer_value_arg(runtime, *value, index),
        None => Ok(default),
    }
}

fn integer_value_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<i64, NativeError> {
    value
        .to_integer_exact()
        .or_else(|| string_to_integer_exact(runtime, value))
        .ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "integer",
            }
            .into(),
        )
}

fn string_to_integer_exact(runtime: &dyn NativeRuntime, value: Value) -> Option<i64> {
    let bytes = runtime.string_bytes(value)?;
    parse_standard_number(bytes)?.to_integer_exact()
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
) -> Result<Cow<'_, [u8]>, NativeError> {
    if let Some(bytes) = runtime.string_bytes(value) {
        return Ok(Cow::Borrowed(bytes));
    }
    if let Some(bytes) = number_arg_bytes(value) {
        return Ok(Cow::Owned(bytes));
    }
    Err(NativeErrorKind::TypeError {
        index,
        expected: "string",
    }
    .into())
}

pub(super) fn number_arg_bytes(value: Value) -> Option<Vec<u8>> {
    if let Some(integer) = value.as_integer() {
        return Some(integer.to_string().into_bytes());
    }
    value.as_float().map(float_arg_bytes)
}

fn float_arg_bytes(value: f64) -> Vec<u8> {
    let mut text = value.to_string();
    if value.is_finite() && !text.contains(['.', 'e', 'E']) {
        text.push_str(".0");
    }
    text.into_bytes()
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{
        STRING_NATIVE_FUNCTIONS, string_byte, string_char, string_dump, string_len, string_lower,
        string_rep, string_reverse, string_sub, string_upper,
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

        fn dump_lua_function(
            &self,
            function: Value,
            strip_debug: bool,
        ) -> Result<Option<Vec<u8>>, NativeError> {
            Ok(function
                .as_closure_index()
                .map(|index| format!("dump:{index}:{strip_debug}").into_bytes()))
        }
    }

    #[test]
    fn string_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = STRING_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "byte")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "char")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "dump")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "find")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "format")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "gmatch")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "gsub")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "len")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "lower")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "match")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "pack")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "packsize")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "rep")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "reverse")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "sub")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "unpack")));
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
            string_len(&mut runtime, &[Value::boolean(false)])
                .expect_err("non-string should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string"
            }
        );
    }

    #[test]
    fn string_functions_coerce_numbers_to_strings() {
        assert_eq!(
            string_len(&mut TestRuntime::default(), &[Value::integer(123)])
                .expect("number len should pass"),
            vec![Value::integer(3)]
        );

        assert_eq!(
            string_byte(&mut TestRuntime::default(), &[Value::integer(65)])
                .expect("number byte should pass"),
            vec![Value::integer(54)]
        );

        assert_eq!(
            string_len(&mut TestRuntime::default(), &[Value::float(1.0)])
                .expect("float len should pass"),
            vec![Value::integer(3)]
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
    fn string_integer_arguments_coerce_exact_numbers_and_numeric_strings() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcd");
        let start = runtime.push_string(b"0x2");
        let end = runtime.push_string(b"0x1p1");
        let count = runtime.push_string(b"2.0");
        let char_code = runtime.push_string(b"0x41");
        let non_integral = runtime.push_string(b"1.5");

        assert_eq!(
            string_byte(&mut runtime, &[subject, start, end]).expect("byte should pass"),
            vec![Value::integer(98)]
        );
        let repeated = string_rep(&mut runtime, &[subject, count]).expect("rep should pass");
        assert_eq!(
            runtime.short_string_bytes(repeated[0]),
            Some(b"abcdabcd".as_slice())
        );
        let char_value =
            string_char(&mut runtime, &[Value::float(65.0), char_code]).expect("char should pass");
        assert_eq!(
            runtime.short_string_bytes(char_value[0]),
            Some(b"AA".as_slice())
        );
        assert_eq!(
            string_byte(&mut runtime, &[subject, non_integral])
                .expect_err("non-integral position should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "integer"
            }
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
    fn string_char_converts_integer_arguments_to_bytes() {
        let mut runtime = TestRuntime::default();
        let value = string_char(
            &mut runtime,
            &[Value::integer(65), Value::integer(0), Value::integer(67)],
        )
        .expect("char should pass");

        assert_eq!(
            runtime.short_string_bytes(value[0]),
            Some(b"A\0C".as_slice())
        );
    }

    #[test]
    fn string_char_allows_empty_argument_list() {
        let mut runtime = TestRuntime::default();
        let value = string_char(&mut runtime, &[]).expect("char should pass");

        assert_eq!(runtime.short_string_bytes(value[0]), Some(b"".as_slice()));
    }

    #[test]
    fn string_dump_uses_runtime_lua_function_dump() {
        let mut runtime = TestRuntime::default();

        let result =
            string_dump(&mut runtime, &[Value::closure_index(7)]).expect("dump should pass");

        assert_eq!(
            runtime.short_string_bytes(result[0]),
            Some(b"dump:7:false".as_slice())
        );
    }

    #[test]
    fn string_dump_uses_lua_truthiness_for_strip_flag() {
        let mut runtime = TestRuntime::default();

        let nil_result = string_dump(&mut runtime, &[Value::closure_index(7), Value::nil()])
            .expect("dump should pass");
        let false_result = string_dump(
            &mut runtime,
            &[Value::closure_index(7), Value::boolean(false)],
        )
        .expect("dump should pass");
        let true_result = string_dump(&mut runtime, &[Value::closure_index(7), Value::integer(0)])
            .expect("dump should pass");

        assert_eq!(
            runtime.short_string_bytes(nil_result[0]),
            Some(b"dump:7:false".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(false_result[0]),
            Some(b"dump:7:false".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(true_result[0]),
            Some(b"dump:7:true".as_slice())
        );
    }

    #[test]
    fn string_dump_rejects_non_lua_functions() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            string_dump(&mut runtime, &[Value::native_function_index(1)])
                .expect_err("native function should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "Lua function"
            }
        );
    }

    #[test]
    fn string_char_rejects_out_of_range_bytes() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            string_char(&mut runtime, &[Value::integer(256)])
                .expect_err("out-of-range byte should fail")
                .kind(),
            &NativeErrorKind::ArgumentOutOfRange { index: 1 }
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
    fn string_rep_treats_nil_separator_as_default() {
        let mut runtime = TestRuntime::default();
        let value = runtime.push_string(b"ab");
        let repeated = string_rep(&mut runtime, &[value, Value::integer(2), Value::nil()])
            .expect("rep should pass");

        assert_eq!(
            runtime.short_string_bytes(repeated[0]),
            Some(b"abab".as_slice())
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
