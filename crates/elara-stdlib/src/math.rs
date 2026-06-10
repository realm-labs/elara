//! Executable math-library natives.

use elara_core::{LuaFloat, Value, float_to_integer_exact};

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable math-library functions currently implemented.
pub const MATH_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "abs"), math_abs),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "ceil"), math_ceil),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "floor"), math_floor),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "max"), math_max),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "min"), math_min),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "sqrt"), math_sqrt),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "type"), math_type),
];

fn math_abs(_runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
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

fn math_floor(_runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
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

fn math_ceil(_runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
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

fn math_sqrt(_runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(args, 1)?;
    Ok(vec![Value::float(
        value
            .to_float()
            .expect("number_arg accepted only numbers")
            .sqrt(),
    )])
}

fn math_min(_runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    extrema_arg(args, Extrema::Min).map(|value| vec![value])
}

fn math_max(_runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    extrema_arg(args, Extrema::Max).map(|value| vec![value])
}

fn math_type(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let result = if value.as_integer().is_some() {
        runtime.intern_short_string(b"integer")?
    } else if value.as_float().is_some() {
        runtime.intern_short_string(b"float")?
    } else {
        Value::nil()
    };
    Ok(vec![result])
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

#[derive(Clone, Copy)]
enum Extrema {
    Min,
    Max,
}

fn extrema_arg(args: &[Value], extrema: Extrema) -> Result<Value, NativeError> {
    if args.is_empty() {
        return Err(NativeErrorKind::MissingArgument { index: 1 }.into());
    }

    let mut selected = number_arg(args, 1)?;
    for index in 2..=args.len() {
        let candidate = number_arg(args, index)?;
        let candidate_float = candidate
            .to_float()
            .expect("number_arg accepted only numbers");
        let selected_float = selected
            .to_float()
            .expect("number_arg accepted only numbers");
        let replace = match extrema {
            Extrema::Min => candidate_float < selected_float,
            Extrema::Max => selected_float < candidate_float,
        };
        if replace {
            selected = candidate;
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use elara_core::{LuaInteger, Value};

    use super::{
        MATH_NATIVE_FUNCTIONS, math_abs, math_ceil, math_floor, math_max, math_min, math_sqrt,
        math_type,
    };
    use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Ok(Value::table_index(index))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_table_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    fn call(function: crate::NativeStdFunction, args: &[Value]) -> Vec<Value> {
        function(&mut TestRuntime::default(), args).expect("native should pass")
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
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "max")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "min")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "sqrt")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "type")));
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
    fn math_min_and_max_return_selected_original_value() {
        assert_eq!(
            call(
                math_min,
                &[Value::integer(3), Value::float(1.5), Value::integer(2)]
            ),
            vec![Value::float(1.5)]
        );
        assert_eq!(
            call(
                math_max,
                &[Value::float(3.5), Value::integer(7), Value::float(7.0)]
            ),
            vec![Value::integer(7)]
        );
    }

    #[test]
    fn math_natives_report_argument_errors() {
        let mut runtime = TestRuntime::default();
        let error = math_abs(&mut runtime, &[]).expect_err("missing argument should fail");
        assert_eq!(error.kind(), &NativeErrorKind::MissingArgument { index: 1 });

        let error =
            math_abs(&mut runtime, &[Value::nil()]).expect_err("non-number argument should fail");
        assert_eq!(
            error.kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "number"
            }
        );

        let error = math_min(&mut runtime, &[Value::integer(1), Value::nil()])
            .expect_err("non-number arg should fail");
        assert_eq!(
            error.kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "number"
            }
        );
    }

    #[test]
    fn math_type_reports_numeric_subtype() {
        let mut runtime = TestRuntime::default();

        let integer = math_type(&mut runtime, &[Value::integer(7)]).expect("type should pass");
        let float = math_type(&mut runtime, &[Value::float(7.0)]).expect("type should pass");

        assert_eq!(
            runtime.short_string_bytes(integer[0]),
            Some(b"integer".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(float[0]),
            Some(b"float".as_slice())
        );
    }

    #[test]
    fn math_type_returns_nil_for_non_numbers() {
        assert_eq!(
            call(math_type, &[Value::boolean(false)]),
            vec![Value::nil()]
        );
    }
}
