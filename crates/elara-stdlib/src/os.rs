//! Executable operating-system library natives.

use std::{
    fs,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
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
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "getenv"), os_getenv),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "remove"), os_remove),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "rename"), os_rename),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "time"), os_time),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "tmpname"), os_tmpname),
];

static CLOCK_START: OnceLock<Instant> = OnceLock::new();
static TMPNAME_COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn os_getenv(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let name = string_arg(runtime, args, 1)?;
    let name = std::str::from_utf8(name).map_err(|_| NativeErrorKind::TypeError {
        index: 1,
        expected: "utf-8 string",
    })?;
    match std::env::var(name) {
        Ok(value) => Ok(vec![runtime.intern_short_string(value.as_bytes())?]),
        Err(std::env::VarError::NotPresent) => Ok(vec![Value::nil()]),
        Err(std::env::VarError::NotUnicode(_)) => Err(NativeErrorKind::RuntimeError {
            message: "environment variable is not valid Unicode".into(),
        }
        .into()),
    }
}

fn os_remove(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let filename = utf8_string_arg(runtime, args, 1)?.to_owned();
    let result = fs::symlink_metadata(&filename).and_then(|metadata| {
        if metadata.is_dir() {
            fs::remove_dir(&filename)
        } else {
            fs::remove_file(&filename)
        }
    });
    file_result(runtime, result, Some(&filename))
}

fn os_rename(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let from = utf8_string_arg(runtime, args, 1)?.to_owned();
    let to = utf8_string_arg(runtime, args, 2)?.to_owned();

    file_result(runtime, fs::rename(from, to), None)
}

fn os_tmpname(runtime: &mut dyn NativeRuntime, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let pid = std::process::id();
    for _ in 0..128 {
        let count = TMPNAME_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = format!("elrtmp_{pid:x}_{count:x}");
        if !std::path::Path::new(&candidate).exists() {
            return Ok(vec![runtime.intern_short_string(candidate.as_bytes())?]);
        }
    }

    Err(NativeErrorKind::RuntimeError {
        message: "unable to generate a unique filename".into(),
    }
    .into())
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

fn utf8_string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<&'a str, NativeError> {
    let bytes = string_arg(runtime, args, index)?;
    std::str::from_utf8(bytes)
        .map_err(|_| NativeErrorKind::TypeError {
            index,
            expected: "utf-8 string",
        })
        .map_err(Into::into)
}

fn string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<&'a [u8], NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    runtime
        .short_string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string",
        })
        .map_err(Into::into)
}

fn file_result(
    runtime: &mut dyn NativeRuntime,
    result: std::io::Result<()>,
    filename: Option<&str>,
) -> Result<Vec<Value>, NativeError> {
    match result {
        Ok(()) => Ok(vec![Value::boolean(true)]),
        Err(error) => {
            let code = error.raw_os_error().unwrap_or(0);
            let message = if let Some(filename) = filename {
                format!("{filename}: {error}")
            } else {
                error.to_string()
            };
            Ok(vec![
                Value::nil(),
                runtime.intern_short_string(message.as_bytes())?,
                Value::integer(i64::from(code)),
            ])
        }
    }
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
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use elara_core::Value;

    use crate::{NativeErrorKind, StdLib, native_functions};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::closure_index(index)
        }

        fn bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    impl crate::NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, crate::NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            self.bytes(value)
        }
    }

    #[test]
    fn os_difftime_returns_numeric_difference() {
        let function = function("difftime");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[Value::integer(20), Value::integer(8)]),
            Ok(vec![Value::float(12.0)])
        );
    }

    #[test]
    fn os_difftime_validates_time_arguments() {
        let function = function("difftime");
        let mut runtime = TestRuntime::default();

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
        let mut runtime = TestRuntime::default();
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
        let mut runtime = TestRuntime::default();

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
    fn os_getenv_validates_name_argument() {
        let function = function("getenv");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing name")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string name")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
    }

    #[test]
    fn os_getenv_returns_nil_for_absent_variable() {
        let function = function("getenv");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"__ELARA_ENV_VAR_THAT_SHOULD_NOT_EXIST__");

        assert_eq!(function(&mut runtime, &[name]), Ok(vec![Value::nil()]));
    }

    #[test]
    fn os_remove_removes_existing_file() {
        let function = function("remove");
        let mut runtime = TestRuntime::default();
        let path = unique_temp_path("remove");
        fs::write(&path, b"temporary").expect("test file should be written");
        let path_value = runtime.push_string(path.to_string_lossy().as_bytes());

        assert_eq!(
            function(&mut runtime, &[path_value]).expect("os.remove should pass"),
            vec![Value::boolean(true)]
        );
        assert!(!path.exists());
    }

    #[test]
    fn os_remove_returns_file_result_for_absent_file() {
        let function = function("remove");
        let mut runtime = TestRuntime::default();
        let path = unique_temp_path("missing");
        let path_value = runtime.push_string(path.to_string_lossy().as_bytes());

        let result = function(&mut runtime, &[path_value]).expect("os.remove should pass");

        assert_eq!(result[0], Value::nil());
        assert!(
            runtime
                .bytes(result[1])
                .is_some_and(|message| { message.starts_with(path.to_string_lossy().as_bytes()) })
        );
        assert!(result[2].as_integer().is_some_and(|code| code != 0));
    }

    #[test]
    fn os_remove_validates_filename_argument() {
        let function = function("remove");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing filename")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string filename")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
    }

    #[test]
    fn os_rename_renames_existing_file() {
        let function = function("rename");
        let mut runtime = TestRuntime::default();
        let from = unique_temp_path("rename-from");
        let to = unique_temp_path("rename-to");
        fs::write(&from, b"temporary").expect("test file should be written");
        let from_value = runtime.push_string(from.to_string_lossy().as_bytes());
        let to_value = runtime.push_string(to.to_string_lossy().as_bytes());

        assert_eq!(
            function(&mut runtime, &[from_value, to_value]).expect("os.rename should pass"),
            vec![Value::boolean(true)]
        );
        assert!(!from.exists());
        assert_eq!(
            fs::read(&to).expect("renamed file should exist"),
            b"temporary"
        );

        fs::remove_file(to).expect("test file should be cleaned up");
    }

    #[test]
    fn os_rename_returns_file_result_for_absent_source() {
        let function = function("rename");
        let mut runtime = TestRuntime::default();
        let from = unique_temp_path("rename-missing");
        let to = unique_temp_path("rename-destination");
        let from_value = runtime.push_string(from.to_string_lossy().as_bytes());
        let to_value = runtime.push_string(to.to_string_lossy().as_bytes());

        let result =
            function(&mut runtime, &[from_value, to_value]).expect("os.rename should pass");

        assert_eq!(result[0], Value::nil());
        assert!(
            runtime
                .bytes(result[1])
                .is_some_and(|message| !message.starts_with(from.to_string_lossy().as_bytes()))
        );
        assert!(result[2].as_integer().is_some_and(|code| code != 0));
        assert!(!to.exists());
    }

    #[test]
    fn os_rename_validates_arguments() {
        let function = function("rename");
        let mut runtime = TestRuntime::default();
        let from = runtime.push_string(b"from");

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing source filename")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[from])
                .expect_err("missing destination filename")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 2 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1), from])
                .expect_err("non-string source filename")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
        assert_eq!(
            function(&mut runtime, &[from, Value::integer(1)])
                .expect_err("non-string destination filename")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "string",
            }
        );
    }

    #[test]
    fn os_tmpname_returns_short_nonexistent_filename() {
        let function = function("tmpname");
        let mut runtime = TestRuntime::default();

        let first = function(&mut runtime, &[]).expect("os.tmpname should pass");
        let second = function(&mut runtime, &[]).expect("os.tmpname should pass");

        let first_name = runtime
            .bytes(first[0])
            .expect("os.tmpname should return a string");
        let second_name = runtime
            .bytes(second[0])
            .expect("os.tmpname should return a string");
        assert!(first_name.starts_with(b"elrtmp_"));
        assert!(second_name.starts_with(b"elrtmp_"));
        assert_ne!(first_name, second_name);
        assert!(
            !std::path::Path::new(std::str::from_utf8(first_name).expect("tmpname is utf-8"))
                .exists()
        );
    }

    #[test]
    fn os_clock_returns_nonnegative_elapsed_seconds() {
        let function = function("clock");
        let mut runtime = TestRuntime::default();

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

    fn unique_temp_path(label: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test time should be after Unix epoch")
            .as_nanos();
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("elara-os-{label}-{suffix}-{count}"))
    }

    fn assert_time_in_range(value: Value, before: i64, after: i64) {
        let value = value.as_integer().expect("os.time should return integer");
        assert!(
            (before..=after).contains(&value),
            "expected {value} to be between {before} and {after}"
        );
    }
}
