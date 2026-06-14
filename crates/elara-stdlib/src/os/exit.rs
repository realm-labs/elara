use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

const UNSUPPORTED_EXIT: &str = "os.exit is not supported by this runtime";

pub(super) fn os_exit(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    validate_exit_code(args.first().copied())?;
    validate_close(args.get(1).copied())?;
    Err(NativeErrorKind::RuntimeError {
        message: UNSUPPORTED_EXIT.into(),
    }
    .into())
}

fn validate_exit_code(value: Option<Value>) -> Result<(), NativeError> {
    match value {
        None => Ok(()),
        Some(value) if value.is_nil() => Ok(()),
        Some(value) if value.as_bool().is_some() => Ok(()),
        Some(value) if value.as_integer().is_some() => Ok(()),
        Some(_) => Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "boolean or integer",
        }
        .into()),
    }
}

fn validate_close(value: Option<Value>) -> Result<(), NativeError> {
    match value {
        None => Ok(()),
        Some(value) if value.is_nil() => Ok(()),
        Some(value) if value.as_bool().is_some() => Ok(()),
        Some(_) => Err(NativeErrorKind::TypeError {
            index: 2,
            expected: "boolean",
        }
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{NativeError, NativeErrorKind, NativeRuntime, native_functions};

    use super::os_exit;

    #[derive(Default)]
    struct TestRuntime;

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, NativeError> {
            unreachable!("os.exit should not allocate strings")
        }

        fn short_string_bytes(&self, _value: Value) -> Option<&[u8]> {
            None
        }
    }

    #[test]
    fn os_exit_reports_unsupported_process_termination() {
        let mut runtime = TestRuntime;

        let error = os_exit(&mut runtime, &[Value::integer(0), Value::boolean(true)])
            .expect_err("os.exit should not terminate the host process");

        assert!(matches!(error.kind(), NativeErrorKind::RuntimeError { .. }));
        assert_eq!(error.message(), "os.exit is not supported by this runtime");
    }

    #[test]
    fn os_exit_validates_arguments() {
        let mut runtime = TestRuntime;

        for args in [
            Vec::new(),
            vec![Value::nil()],
            vec![Value::boolean(true)],
            vec![Value::boolean(false), Value::boolean(false)],
            vec![Value::integer(7), Value::nil()],
        ] {
            assert_eq!(
                os_exit(&mut runtime, &args)
                    .expect_err("valid os.exit args still cannot exit")
                    .kind(),
                &NativeErrorKind::RuntimeError {
                    message: "os.exit is not supported by this runtime".into(),
                }
            );
        }

        assert_eq!(
            os_exit(&mut runtime, &[Value::float(1.5)])
                .expect_err("non-integer status should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "boolean or integer",
            }
        );
        assert_eq!(
            os_exit(&mut runtime, &[Value::integer(0), Value::integer(1)])
                .expect_err("non-boolean close flag should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "boolean",
            }
        );
    }

    #[test]
    fn os_exit_is_discoverable() {
        assert!(native_functions(crate::StdLib::Os).iter().any(|function| {
            function.descriptor() == crate::FunctionSpec::new(crate::StdLib::Os, "exit")
        }));
    }
}
