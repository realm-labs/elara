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

    let Some(format) = format.strip_prefix('!') else {
        return Err(unsupported_format_error());
    };
    let date = utc_date_time(seconds);

    if format == "*t" {
        return date_table(runtime, date);
    }

    let formatted = format_utc_date(format, date)?;
    Ok(vec![runtime.intern_string(formatted.as_bytes())?])
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

fn format_utc_date(format: &str, date: UtcDateTime) -> Result<String, NativeError> {
    let mut output = String::new();
    let mut chars = format.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        let Some(specifier) = chars.next() else {
            return Err(invalid_conversion_error(""));
        };
        push_date_specifier(&mut output, date, specifier)?;
    }
    Ok(output)
}

fn push_date_specifier(
    output: &mut String,
    date: UtcDateTime,
    specifier: char,
) -> Result<(), NativeError> {
    match specifier {
        '%' => output.push('%'),
        'C' => output.push_str(&format!("{:02}", date.year.div_euclid(100))),
        'D' => {
            push_date_specifier(output, date, 'm')?;
            output.push('/');
            push_date_specifier(output, date, 'd')?;
            output.push('/');
            push_date_specifier(output, date, 'y')?;
        }
        'Y' => output.push_str(&format!("{:04}", date.year)),
        'y' => output.push_str(&format!("{:02}", date.year.rem_euclid(100))),
        'm' => output.push_str(&format!("{:02}", date.month)),
        'd' => output.push_str(&format!("{:02}", date.day)),
        'e' => output.push_str(&format!("{:2}", date.day)),
        'H' => output.push_str(&format!("{:02}", date.hour)),
        'I' => output.push_str(&format!("{:02}", hour_12(date.hour))),
        'M' => output.push_str(&format!("{:02}", date.min)),
        'S' => output.push_str(&format!("{:02}", date.sec)),
        'j' => output.push_str(&format!("{:03}", date.yday)),
        'w' => output.push_str(&(date.wday - 1).to_string()),
        'n' => output.push('\n'),
        'p' => output.push_str(if date.hour < 12 { "AM" } else { "PM" }),
        'r' => {
            push_date_specifier(output, date, 'I')?;
            output.push(':');
            push_date_specifier(output, date, 'M')?;
            output.push(':');
            push_date_specifier(output, date, 'S')?;
            output.push(' ');
            push_date_specifier(output, date, 'p')?;
        }
        'R' => {
            push_date_specifier(output, date, 'H')?;
            output.push(':');
            push_date_specifier(output, date, 'M')?;
        }
        't' => output.push('\t'),
        'a' => output.push_str(WEEKDAY_ABBR[(date.wday - 1) as usize]),
        'A' => output.push_str(WEEKDAY_NAME[(date.wday - 1) as usize]),
        'b' | 'h' => output.push_str(MONTH_ABBR[(date.month - 1) as usize]),
        'B' => output.push_str(MONTH_NAME[(date.month - 1) as usize]),
        'F' => {
            push_date_specifier(output, date, 'Y')?;
            output.push('-');
            push_date_specifier(output, date, 'm')?;
            output.push('-');
            push_date_specifier(output, date, 'd')?;
        }
        'T' => {
            push_date_specifier(output, date, 'H')?;
            output.push(':');
            push_date_specifier(output, date, 'M')?;
            output.push(':');
            push_date_specifier(output, date, 'S')?;
        }
        _ => return Err(invalid_conversion_error(&specifier.to_string())),
    }
    Ok(())
}

fn hour_12(hour: i64) -> i64 {
    match hour.rem_euclid(12) {
        0 => 12,
        hour => hour,
    }
}

const WEEKDAY_ABBR: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAY_NAME: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTH_NAME: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn unsupported_format_error() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "os.date currently supports only UTC formats prefixed with '!'".into(),
    }
    .into()
}

fn invalid_conversion_error(specifier: &str) -> NativeError {
    NativeErrorKind::RuntimeError {
        message: format!("invalid conversion specifier '%{specifier}'").into_boxed_str(),
    }
    .into()
}
