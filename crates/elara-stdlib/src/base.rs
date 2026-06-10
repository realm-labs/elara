//! Executable base-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable base-library functions currently implemented.
pub const BASE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "assert"), base_assert),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawequal"), base_rawequal),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "select"), base_select),
];

fn base_assert(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let condition = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if is_truthy(condition) {
        Ok(args.to_vec())
    } else {
        Err(NativeErrorKind::LuaError.into())
    }
}

fn base_rawequal(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let left = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let right = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    Ok(vec![Value::boolean(left == right)])
}

fn base_select(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let index =
        args.first()
            .and_then(|value| value.as_integer())
            .ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "integer",
            })?;
    let value_count = args.len().saturating_sub(1);
    let start = select_start(index, value_count)?;
    Ok(args[start..].to_vec())
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

fn select_start(index: i64, value_count: usize) -> Result<usize, NativeError> {
    let value_count = i64::try_from(value_count).expect("argument count must fit in i64");
    let normalized = if index < 0 {
        value_count + index + 1
    } else if index > value_count {
        value_count + 1
    } else {
        index
    };
    if normalized < 1 {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 1 }.into());
    }
    usize::try_from(normalized).map_err(|_| NativeErrorKind::ArgumentOutOfRange { index: 1 }.into())
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{BASE_NATIVE_FUNCTIONS, base_assert, base_rawequal, base_select};
    use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

    struct TestRuntime;

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, NativeError> {
            unimplemented!("base native tests do not allocate strings")
        }

        fn short_string_bytes(&self, _value: Value) -> Option<&[u8]> {
            None
        }
    }

    fn call(function: crate::NativeStdFunction, args: &[Value]) -> Vec<Value> {
        function(&mut TestRuntime, args).expect("native should pass")
    }

    #[test]
    fn base_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = BASE_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "assert")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawequal")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "select")));
    }

    #[test]
    fn base_assert_returns_all_arguments_when_truthy() {
        assert_eq!(
            call(
                base_assert,
                &[Value::boolean(true), Value::integer(7), Value::nil()]
            ),
            vec![Value::boolean(true), Value::integer(7), Value::nil()]
        );
    }

    #[test]
    fn base_assert_errors_when_false_or_nil() {
        assert_eq!(
            base_assert(&mut TestRuntime, &[Value::boolean(false)])
                .expect_err("false assert should fail")
                .kind(),
            &NativeErrorKind::LuaError
        );
        assert_eq!(
            base_assert(&mut TestRuntime, &[Value::nil()])
                .expect_err("nil assert should fail")
                .kind(),
            &NativeErrorKind::LuaError
        );
    }

    #[test]
    fn base_rawequal_compares_raw_values() {
        assert_eq!(
            call(base_rawequal, &[Value::integer(7), Value::float(7.0)]),
            vec![Value::boolean(true)]
        );
        assert_eq!(
            call(base_rawequal, &[Value::boolean(true), Value::integer(1)]),
            vec![Value::boolean(false)]
        );
    }

    #[test]
    fn base_select_returns_positioned_arguments() {
        let values = [
            Value::integer(2),
            Value::integer(10),
            Value::integer(20),
            Value::integer(30),
        ];
        assert_eq!(
            call(base_select, &values),
            vec![Value::integer(20), Value::integer(30)]
        );

        let values = [
            Value::integer(-1),
            Value::integer(10),
            Value::integer(20),
            Value::integer(30),
        ];
        assert_eq!(call(base_select, &values), vec![Value::integer(30)]);
    }

    #[test]
    fn base_select_reports_bad_position() {
        assert_eq!(
            base_select(&mut TestRuntime, &[Value::integer(0)])
                .expect_err("zero select should fail")
                .kind(),
            &NativeErrorKind::ArgumentOutOfRange { index: 1 }
        );
    }
}
