//! Executable math-library natives.

use elara_core::{LuaFloat, Value, float_to_integer_exact};

use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, StdLib};

/// Executable math-library functions currently implemented.
pub const MATH_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "abs"), math_abs),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "ceil"), math_ceil),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "floor"), math_floor),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "sqrt"), math_sqrt),
];

fn math_abs(args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(args, 1)?;
    let result = if let Some(integer) = value.as_integer() {
        if integer < 0 {
            Value::integer(integer.wrapping_neg())
        } else {
            value
        }
    } else {
        Value::float(
            value
                .as_float()
                .expect("number_arg accepted only numbers")
                .abs(),
        )
    };
    Ok(vec![result])
}

fn math_floor(args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(args, 1)?;
    let result = if value.as_integer().is_some() {
        value
    } else {
        number_result(
            value
                .to_float()
                .expect("number_arg accepted only numbers")
                .floor(),
        )
    };
    Ok(vec![result])
}

fn math_ceil(args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(args, 1)?;
    let result = if value.as_integer().is_some() {
        value
    } else {
        number_result(
            value
                .to_float()
                .expect("number_arg accepted only numbers")
                .ceil(),
        )
    };
    Ok(vec![result])
}

fn math_sqrt(args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(args, 1)?;
    Ok(vec![Value::float(
        value
            .to_float()
            .expect("number_arg accepted only numbers")
            .sqrt(),
    )])
}

fn number_arg(args: &[Value], index: usize) -> Result<Value, NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    if value.is_number() {
        Ok(value)
    } else {
        Err(NativeErrorKind::TypeError {
            index,
            expected: "number",
        }
        .into())
    }
}

fn number_result(value: LuaFloat) -> Value {
    float_to_integer_exact(value).map_or_else(|| Value::float(value), Value::integer)
}

#[cfg(test)]
mod tests {
    use elara_core::{LuaInteger, Value};

    use super::{MATH_NATIVE_FUNCTIONS, math_abs, math_ceil, math_floor, math_sqrt};
    use crate::{FunctionSpec, NativeErrorKind, StdLib};

    fn call(function: crate::NativeStdFunction, args: &[Value]) -> Vec<Value> {
        function(args).expect("native should pass")
    }

    #[test]
    fn math_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = MATH_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "abs")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "ceil")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "floor")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "sqrt")));
    }

    #[test]
    fn math_abs_preserves_integer_results() {
        assert_eq!(
            call(math_abs, &[Value::integer(-7)]),
            vec![Value::integer(7)]
        );
        assert_eq!(
            call(math_abs, &[Value::integer(LuaInteger::MIN)]),
            vec![Value::integer(LuaInteger::MIN)]
        );
    }

    #[test]
    fn math_abs_accepts_float_values() {
        assert_eq!(
            call(math_abs, &[Value::float(-2.5)]),
            vec![Value::float(2.5)]
        );
    }

    #[test]
    fn math_floor_and_ceil_preserve_or_convert_integer_results() {
        assert_eq!(
            call(math_floor, &[Value::integer(9)]),
            vec![Value::integer(9)]
        );
        assert_eq!(
            call(math_floor, &[Value::float(3.8)]),
            vec![Value::integer(3)]
        );
        assert_eq!(
            call(math_ceil, &[Value::float(3.2)]),
            vec![Value::integer(4)]
        );
    }

    #[test]
    fn math_sqrt_returns_float() {
        assert_eq!(
            call(math_sqrt, &[Value::integer(9)]),
            vec![Value::float(3.0)]
        );
    }

    #[test]
    fn math_natives_report_argument_errors() {
        let error = math_abs(&[]).expect_err("missing argument should fail");
        assert_eq!(error.kind(), &NativeErrorKind::MissingArgument { index: 1 });

        let error = math_abs(&[Value::nil()]).expect_err("non-number argument should fail");
        assert_eq!(
            error.kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "number"
            }
        );
    }
}
