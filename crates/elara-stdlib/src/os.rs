//! Executable operating-system library natives.

use std::{
    sync::OnceLock,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable `os` library functions currently implemented.
pub const OS_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "clock"), os_clock),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "difftime"), os_difftime),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "time"), os_time),
];

static CLOCK_START: OnceLock<Instant> = OnceLock::new();

fn os_clock(_runtime: &mut dyn NativeRuntime, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let start = CLOCK_START.get_or_init(Instant::now);
    Ok(vec![Value::float(start.elapsed().as_secs_f64())])
}

fn os_difftime(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let first = time_arg(args, 1)?;
    let second = time_arg(args, 2)?;
    Ok(vec![Value::float((first - second) as f64)])
}

fn os_time(_runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    match args.first().copied() {
        None => current_unix_time(),
        Some(value) if value.is_nil() => current_unix_time(),
        Some(value) if value.is_table() => Err(NativeErrorKind::RuntimeError {
            message: "os.time date table form is not implemented".into(),
        }
        .into()),
        Some(_) => Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "table",
        }
        .into()),
    }
}

fn current_unix_time() -> Result<Vec<Value>, NativeError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| NativeErrorKind::RuntimeError {
            message: format!("system time before Unix epoch: {error}").into_boxed_str(),
        })?
        .as_secs();
    let seconds = i64::try_from(seconds).map_err(|_| NativeErrorKind::RuntimeError {
        message: "system time cannot be represented as Lua integer".into(),
    })?;
    Ok(vec![Value::integer(seconds)])
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let function = function("difftime");
        let mut runtime = TestRuntime;

        assert_eq!(
            function(&mut runtime, &[Value::integer(20), Value::integer(8)]),
            Ok(vec![Value::float(12.0)])
        );
    }

    #[test]
    fn os_difftime_validates_time_arguments() {
        let function = function("difftime");
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

    #[test]
    fn os_time_without_table_returns_current_unix_time() {
        let function = function("time");
        let mut runtime = TestRuntime;
        let before = current_unix_seconds();

        let without_args = function(&mut runtime, &[]).expect("os.time should pass");
        let with_nil = function(&mut runtime, &[Value::nil()]).expect("os.time(nil) should pass");
        let after = current_unix_seconds();

        assert_time_in_range(without_args[0], before, after);
        assert_time_in_range(with_nil[0], before, after);
    }

    #[test]
    fn os_time_rejects_unsupported_date_table_form() {
        let function = function("time");
        let mut runtime = TestRuntime;

        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-table time argument")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "table",
            }
        );
        assert_eq!(
            function(&mut runtime, &[Value::table_index(0)])
                .expect_err("table time form is not implemented")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "os.time date table form is not implemented".into(),
            }
        );
    }

    #[test]
    fn os_clock_returns_nonnegative_elapsed_seconds() {
        let function = function("clock");
        let mut runtime = TestRuntime;

        let first = function(&mut runtime, &[]).expect("os.clock should pass")[0]
            .as_float()
            .expect("os.clock should return float");
        let second = function(&mut runtime, &[]).expect("os.clock should pass")[0]
            .as_float()
            .expect("os.clock should return float");

        assert!(first >= 0.0);
        assert!(second >= first);
    }

    fn function(name: &str) -> crate::NativeStdFunction {
        native_functions(StdLib::Os)
            .iter()
            .find(|function| function.descriptor().name() == name)
            .expect("os native function should exist")
            .function()
    }

    fn current_unix_seconds() -> i64 {
        i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test system time should be after Unix epoch")
                .as_secs(),
        )
        .expect("test system time should fit in i64")
    }

    fn assert_time_in_range(value: Value, before: i64, after: i64) {
        let value = value.as_integer().expect("os.time should return integer");
        assert!(
            (before..=after).contains(&value),
            "expected {value} to be between {before} and {after}"
        );
    }
}
