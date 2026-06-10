//! Executable string-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable string-library functions currently implemented.
pub const STRING_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::String, "len"),
    string_len,
)];

fn string_len(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let bytes = runtime
        .short_string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index: 1,
            expected: "string",
        })?;
    let len = i64::try_from(bytes.len()).expect("runtime string length must fit in LuaInteger");
    Ok(vec![Value::integer(len)])
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{STRING_NATIVE_FUNCTIONS, string_len};
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
}
