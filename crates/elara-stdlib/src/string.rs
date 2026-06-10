//! Executable string-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable string-library functions currently implemented.
pub const STRING_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "len"), string_len),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "lower"), string_lower),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::String, "reverse"), string_reverse),
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

    use super::{STRING_NATIVE_FUNCTIONS, string_len, string_lower, string_reverse, string_upper};
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

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "len")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "lower")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::String, "reverse")));
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
}
