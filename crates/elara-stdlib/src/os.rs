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

mod date;
mod execute;
mod exit;
mod time;

use self::{
    date::os_date,
    execute::os_execute,
    exit::os_exit,
    time::{utc_date_time, utc_seconds_from_civil_time},
};

/// Executable `os` library functions currently implemented.
pub const OS_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "clock"), os_clock),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "date"), os_date),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "difftime"), os_difftime),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "execute"), os_execute),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "exit"), os_exit),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "getenv"), os_getenv),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "remove"), os_remove),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "rename"), os_rename),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "setlocale"), os_setlocale),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "time"), os_time),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Os, "tmpname"), os_tmpname),
];

static CLOCK_START: OnceLock<Instant> = OnceLock::new();
static TMPNAME_COUNTER: AtomicU64 = AtomicU64::new(0);
const LOCALE_CATEGORIES: &[&str] = &["all", "collate", "ctype", "monetary", "numeric", "time"];

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

fn os_time(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    match args.first().copied() {
        None => current_unix_time(),
        Some(value) if value.is_nil() => current_unix_time(),
        Some(value) if value.is_table() => time_from_table(runtime, value),
        Some(_) => Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "table",
        }
        .into()),
    }
}

fn time_from_table(
    runtime: &mut dyn NativeRuntime,
    table: Value,
) -> Result<Vec<Value>, NativeError> {
    let year = required_integer_field(runtime, table, b"year")?;
    let month = required_integer_field(runtime, table, b"month")?;
    let day = required_integer_field(runtime, table, b"day")?;
    let hour = optional_integer_field(runtime, table, b"hour", 12)?;
    let min = optional_integer_field(runtime, table, b"min", 0)?;
    let sec = optional_integer_field(runtime, table, b"sec", 0)?;

    let seconds = utc_seconds_from_civil_time(year, month, day, hour, min, sec)?;
    write_normalized_time_fields(runtime, table, seconds)?;
    Ok(vec![Value::integer(seconds)])
}

fn os_getenv(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let name = string_arg(runtime, args, 1)?;
    let name = std::str::from_utf8(name).map_err(|_| NativeErrorKind::TypeError {
        index: 1,
        expected: "utf-8 string",
    })?;
    match std::env::var(name) {
        Ok(value) => Ok(vec![runtime.intern_string(value.as_bytes())?]),
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
            return Ok(vec![runtime.intern_string(candidate.as_bytes())?]);
        }
    }

    Err(NativeErrorKind::RuntimeError {
        message: "unable to generate a unique filename".into(),
    }
    .into())
}

fn os_setlocale(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let locale = optional_utf8_string_arg(runtime, args, 1)?;
    let category = optional_utf8_string_arg(runtime, args, 2)?.unwrap_or("all");
    if !LOCALE_CATEGORIES.contains(&category) {
        return Err(NativeErrorKind::RuntimeError {
            message: format!("invalid locale category '{category}'").into_boxed_str(),
        }
        .into());
    }

    match locale {
        None | Some("C") => Ok(vec![runtime.intern_string(b"C")?]),
        Some(_) => Ok(vec![Value::nil()]),
    }
}

fn current_unix_time() -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::integer(current_unix_seconds()?)])
}

pub(super) fn current_unix_seconds() -> Result<i64, NativeError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| NativeErrorKind::RuntimeError {
            message: format!("system time before Unix epoch: {error}").into_boxed_str(),
        })?
        .as_secs();
    let seconds = i64::try_from(seconds).map_err(|_| NativeErrorKind::RuntimeError {
        message: "system time cannot be represented as Lua integer".into(),
    })?;
    Ok(seconds)
}

fn required_integer_field(
    runtime: &mut dyn NativeRuntime,
    table: Value,
    key: &'static [u8],
) -> Result<i64, NativeError> {
    match table_field(runtime, table, key)? {
        value if value.is_nil() => Err(NativeErrorKind::RuntimeError {
            message: format!(
                "field '{}' missing in date table",
                std::str::from_utf8(key).expect("date field keys are utf-8")
            )
            .into_boxed_str(),
        }
        .into()),
        value => integer_date_field(key, value),
    }
}

fn optional_integer_field(
    runtime: &mut dyn NativeRuntime,
    table: Value,
    key: &'static [u8],
    default: i64,
) -> Result<i64, NativeError> {
    match table_field(runtime, table, key)? {
        value if value.is_nil() => Ok(default),
        value => integer_date_field(key, value),
    }
}

fn integer_date_field(key: &'static [u8], value: Value) -> Result<i64, NativeError> {
    value.as_integer().ok_or_else(|| {
        NativeErrorKind::RuntimeError {
            message: format!(
                "field '{}' is not an integer",
                std::str::from_utf8(key).expect("date field keys are utf-8")
            )
            .into_boxed_str(),
        }
        .into()
    })
}

fn table_field(
    runtime: &mut dyn NativeRuntime,
    table: Value,
    key: &'static [u8],
) -> Result<Value, NativeError> {
    let key = runtime.intern_short_string(key)?;
    runtime.table_get(table, key)
}

fn set_integer_field(
    runtime: &mut dyn NativeRuntime,
    table: Value,
    key: &'static [u8],
    value: i64,
) -> Result<(), NativeError> {
    let key = runtime.intern_short_string(key)?;
    runtime.table_set(table, key, Value::integer(value))
}

fn write_normalized_time_fields(
    runtime: &mut dyn NativeRuntime,
    table: Value,
    seconds: i64,
) -> Result<(), NativeError> {
    let date = utc_date_time(seconds);
    set_integer_field(runtime, table, b"year", date.year)?;
    set_integer_field(runtime, table, b"month", date.month)?;
    set_integer_field(runtime, table, b"day", date.day)?;
    set_integer_field(runtime, table, b"hour", date.hour)?;
    set_integer_field(runtime, table, b"min", date.min)?;
    set_integer_field(runtime, table, b"sec", date.sec)?;
    Ok(())
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

pub(super) fn optional_utf8_string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Option<&'a str>, NativeError> {
    let Some(value) = args.get(index - 1).copied() else {
        return Ok(None);
    };
    if value.is_nil() {
        return Ok(None);
    }
    let bytes = runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string",
        })?;
    std::str::from_utf8(bytes)
        .map(Some)
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
        .string_bytes(value)
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
                runtime.intern_string(message.as_bytes())?,
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

    use crate::{NativeErrorKind, NativeRuntime, StdLib, native_functions};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        tables: Vec<Vec<(Value, Value)>>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::closure_index(index)
        }

        fn intern_string(&mut self, bytes: &[u8]) -> Value {
            if let Some(index) = self
                .strings
                .iter()
                .position(|existing| existing.as_ref() == bytes)
            {
                return Value::closure_index(
                    u32::try_from(index).expect("test string index fits in u32"),
                );
            }
            self.push_string(bytes)
        }

        fn bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }

        fn date_table(&mut self, entries: &[(&'static [u8], i64)]) -> Value {
            let entries = entries
                .iter()
                .map(|(name, value)| (self.intern_string(name), Value::integer(*value)))
                .collect::<Vec<_>>();
            self.create_table(&entries)
                .expect("test runtime should create table")
        }

        fn integer_field(&mut self, table: Value, field: &'static [u8]) -> Option<i64> {
            let key = self.intern_string(field);
            self.table_get(table, key).ok()?.as_integer()
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, crate::NativeError> {
            Ok(self.intern_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            self.bytes(value)
        }

        fn create_table(
            &mut self,
            entries: &[(Value, Value)],
        ) -> Result<Value, crate::NativeError> {
            let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
            self.tables.push(entries.to_vec());
            Ok(Value::table_index(index))
        }

        fn table_get(&self, table: Value, key: Value) -> Result<Value, crate::NativeError> {
            let index = table.as_table_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "table",
            })? as usize;
            Ok(self
                .tables
                .get(index)
                .and_then(|entries| {
                    entries
                        .iter()
                        .find_map(|(candidate, value)| (*candidate == key).then_some(*value))
                })
                .unwrap_or_else(Value::nil))
        }

        fn table_set(
            &mut self,
            table: Value,
            key: Value,
            value: Value,
        ) -> Result<(), crate::NativeError> {
            let index = table.as_table_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "table",
            })? as usize;
            let entries = self
                .tables
                .get_mut(index)
                .ok_or(NativeErrorKind::TypeError {
                    index: 1,
                    expected: "table",
                })?;
            if let Some((_, existing)) = entries.iter_mut().find(|(candidate, _)| *candidate == key)
            {
                *existing = value;
            } else {
                entries.push((key, value));
            }
            Ok(())
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
    fn os_date_returns_utc_table_fields() {
        let function = function("date");
        let mut runtime = TestRuntime::default();
        let format = runtime.push_string(b"!*t");

        let result =
            function(&mut runtime, &[format, Value::integer(0)]).expect("os.date should pass");

        let table = result[0];
        assert_eq!(runtime.integer_field(table, b"year"), Some(1970));
        assert_eq!(runtime.integer_field(table, b"month"), Some(1));
        assert_eq!(runtime.integer_field(table, b"day"), Some(1));
        assert_eq!(runtime.integer_field(table, b"hour"), Some(0));
        assert_eq!(runtime.integer_field(table, b"min"), Some(0));
        assert_eq!(runtime.integer_field(table, b"sec"), Some(0));
        assert_eq!(runtime.integer_field(table, b"wday"), Some(5));
        assert_eq!(runtime.integer_field(table, b"yday"), Some(1));
        let isdst = runtime.intern_string(b"isdst");
        assert_eq!(runtime.table_get(table, isdst), Ok(Value::boolean(false)));
    }

    #[test]
    fn os_date_formats_utc_strings() {
        let function = function("date");
        let mut runtime = TestRuntime::default();
        let format = runtime.push_string(b"!%Y-%m-%d %H:%M:%S %j %w %%");

        let result =
            function(&mut runtime, &[format, Value::integer(0)]).expect("os.date should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(b"1970-01-01 00:00:00 001 4 %".as_slice())
        );
    }

    #[test]
    fn os_date_formats_utc_names_and_aliases() {
        let function = function("date");
        let mut runtime = TestRuntime::default();
        let format = runtime.push_string(b"!%a %A %b %B %F %T");

        let result =
            function(&mut runtime, &[format, Value::integer(0)]).expect("os.date should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(b"Thu Thursday Jan January 1970-01-01 00:00:00".as_slice())
        );
    }

    #[test]
    fn os_date_formats_portable_utc_strftime_specifiers() {
        let function = function("date");
        let mut runtime = TestRuntime::default();
        let format = runtime.push_string(b"!%C %D %e %I %p %r %R %n%t");

        let result = function(&mut runtime, &[format, Value::integer(951868799)])
            .expect("os.date should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(b"20 02/29/00 29 11 PM 11:59:59 PM 23:59 \n\t".as_slice())
        );
    }

    #[test]
    fn os_date_validates_supported_utc_table_subset() {
        let function = function("date");
        let mut runtime = TestRuntime::default();
        let local_format = runtime.push_string(b"%Y");
        let invalid_utc_format = runtime.push_string(b"!%Q");
        let utc_table_format = runtime.push_string(b"!*t");

        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string format")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
        assert_eq!(
            function(&mut runtime, &[utc_table_format, Value::boolean(true)])
                .expect_err("non-integer time")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "integer",
            }
        );
        assert_eq!(
            function(&mut runtime, &[local_format, Value::integer(0)])
                .expect_err("local format")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "os.date currently supports only UTC formats prefixed with '!'".into(),
            }
        );
        assert_eq!(
            function(&mut runtime, &[invalid_utc_format, Value::integer(0)])
                .expect_err("invalid UTC format")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "invalid conversion specifier '%Q'".into(),
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
    fn os_time_converts_date_table_to_utc_unix_time() {
        let function = function("time");
        let mut runtime = TestRuntime::default();
        let table = runtime.date_table(&[
            (b"year", 1970),
            (b"month", 1),
            (b"day", 1),
            (b"hour", 0),
            (b"min", 0),
            (b"sec", 0),
        ]);

        assert_eq!(
            function(&mut runtime, &[table]).expect("os.time table form should pass"),
            vec![Value::integer(0)]
        );
        assert_eq!(runtime.integer_field(table, b"year"), Some(1970));
        assert_eq!(runtime.integer_field(table, b"month"), Some(1));
        assert_eq!(runtime.integer_field(table, b"day"), Some(1));
        assert_eq!(runtime.integer_field(table, b"hour"), Some(0));
    }

    #[test]
    fn os_time_uses_table_defaults_and_normalizes_fields() {
        let function = function("time");
        let mut runtime = TestRuntime::default();
        let table = runtime.date_table(&[(b"year", 1970), (b"month", 13), (b"day", 1)]);

        assert_eq!(
            function(&mut runtime, &[table]).expect("os.time table form should pass"),
            vec![Value::integer(31_579_200)]
        );
        assert_eq!(runtime.integer_field(table, b"year"), Some(1971));
        assert_eq!(runtime.integer_field(table, b"month"), Some(1));
        assert_eq!(runtime.integer_field(table, b"day"), Some(1));
        assert_eq!(
            runtime.integer_field(table, b"hour"),
            Some(12),
            "Lua defaults omitted hour to noon"
        );
    }

    #[test]
    fn os_time_validates_date_table_fields() {
        let function = function("time");
        let mut runtime = TestRuntime::default();
        let missing_day = runtime.date_table(&[(b"year", 1970), (b"month", 1)]);
        let non_integer_year = {
            let key = runtime.intern_string(b"year");
            runtime
                .create_table(&[(key, Value::boolean(true))])
                .expect("test runtime should create table")
        };

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
            function(&mut runtime, &[missing_day])
                .expect_err("missing day")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "field 'day' missing in date table".into(),
            }
        );
        assert_eq!(
            function(&mut runtime, &[non_integer_year])
                .expect_err("non-integer year")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "field 'year' is not an integer".into(),
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
    fn os_setlocale_supports_c_locale_queries() {
        let function = function("setlocale");
        let mut runtime = TestRuntime::default();
        let c_locale = runtime.push_string(b"C");
        let category = runtime.push_string(b"numeric");

        let queried = function(&mut runtime, &[]).expect("os.setlocale query should pass");
        let set = function(&mut runtime, &[c_locale, category]).expect("os.setlocale should pass");

        assert_eq!(runtime.bytes(queried[0]), Some(b"C".as_slice()));
        assert_eq!(runtime.bytes(set[0]), Some(b"C".as_slice()));
    }

    #[test]
    fn os_setlocale_returns_nil_for_unsupported_locale() {
        let function = function("setlocale");
        let mut runtime = TestRuntime::default();
        let locale = runtime.push_string(b"zz_NO");

        assert_eq!(
            function(&mut runtime, &[locale]).expect("os.setlocale should pass"),
            vec![Value::nil()]
        );
    }

    #[test]
    fn os_setlocale_validates_arguments() {
        let function = function("setlocale");
        let mut runtime = TestRuntime::default();
        let c_locale = runtime.push_string(b"C");
        let invalid_category = runtime.push_string(b"invalid");

        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string locale")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
        assert_eq!(
            function(&mut runtime, &[c_locale, Value::integer(1)])
                .expect_err("non-string category")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "string",
            }
        );
        assert_eq!(
            function(&mut runtime, &[c_locale, invalid_category])
                .expect_err("invalid category")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "invalid locale category 'invalid'".into(),
            }
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
