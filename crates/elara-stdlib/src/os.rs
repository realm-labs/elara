//! Executable operating-system library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable `os` library functions currently implemented.
pub const OS_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Os, "difftime"),
    os_difftime,
)];

fn os_difftime(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let first = time_arg(args, 1)?;
    let second = time_arg(args, 2)?;
    Ok(vec![Value::float((first - second) as f64)])
}

fn time_arg(args: &[Value], index: usize) -> Result<i64, NativeError> {
    args.get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?
        .as_integer()
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "integer",
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{NativeErrorKind, StdLib, native_functions};

    #[derive(Default)]
    struct TestRuntime;

    impl crate::NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, crate::NativeError> {
            unreachable!("os.difftime does not intern strings")
        }

        fn short_string_bytes(&self, _value: Value) -> Option<&[u8]> {
            unreachable!("os.difftime does not read strings")
        }
    }

    #[test]
    fn os_difftime_returns_numeric_difference() {
        let function = native_functions(StdLib::Os)[0].function();
        let mut runtime = TestRuntime;

        assert_eq!(
            function(&mut runtime, &[Value::integer(20), Value::integer(8)]),
            Ok(vec![Value::float(12.0)])
        );
    }

    #[test]
    fn os_difftime_validates_time_arguments() {
        let function = native_functions(StdLib::Os)[0].function();
        let mut runtime = TestRuntime;

        assert_eq!(
            function(&mut runtime, &[Value::integer(20)])
                .expect_err("missing second argument")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 2 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::float(20.0), Value::integer(8)])
                .expect_err("non-integer time")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "integer",
            }
        );
    }
}
