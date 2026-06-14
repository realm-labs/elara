use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{
    current_unix_seconds, optional_utf8_string_arg,
    time::{UtcDateTime, utc_date_time},
};

pub(super) fn os_date(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let format = optional_utf8_string_arg(runtime, args, 1)?.unwrap_or("%c");
    let seconds = optional_time_arg(args, 2)?;

    if format == "!*t" {
        return date_table(runtime, utc_date_time(seconds));
    }

    Err(NativeErrorKind::RuntimeError {
        message: "os.date currently supports only UTC table format '!*t'".into(),
    }
    .into())
}

fn date_table(
    runtime: &mut dyn NativeRuntime,
    date: UtcDateTime,
) -> Result<Vec<Value>, NativeError> {
    let entries = [
        (
            runtime.intern_short_string(b"year")?,
            Value::integer(date.year),
        ),
        (
            runtime.intern_short_string(b"month")?,
            Value::integer(date.month),
        ),
        (
            runtime.intern_short_string(b"day")?,
            Value::integer(date.day),
        ),
        (
            runtime.intern_short_string(b"hour")?,
            Value::integer(date.hour),
        ),
        (
            runtime.intern_short_string(b"min")?,
            Value::integer(date.min),
        ),
        (
            runtime.intern_short_string(b"sec")?,
            Value::integer(date.sec),
        ),
        (
            runtime.intern_short_string(b"yday")?,
            Value::integer(date.yday),
        ),
        (
            runtime.intern_short_string(b"wday")?,
            Value::integer(date.wday),
        ),
        (
            runtime.intern_short_string(b"isdst")?,
            Value::boolean(false),
        ),
    ];
    Ok(vec![runtime.create_table(&entries)?])
}

fn optional_time_arg(args: &[Value], index: usize) -> Result<i64, NativeError> {
    match args.get(index - 1).copied() {
        None => current_unix_seconds(),
        Some(value) if value.is_nil() => current_unix_seconds(),
        Some(value) => value
            .as_integer()
            .ok_or(NativeErrorKind::TypeError {
                index,
                expected: "integer",
            })
            .map_err(Into::into),
    }
}
