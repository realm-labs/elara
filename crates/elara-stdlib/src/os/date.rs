use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{
    current_unix_seconds, optional_utf8_string_arg,
    time::{UtcDateTime, days_from_civil, utc_date_time},
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
    let mut chars = format.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        let Some(specifier) = chars.next() else {
            return Err(invalid_conversion_error(""));
        };
        if let Some(modified) = modified_date_specifier(specifier, chars.peek().copied()) {
            chars.next();
            push_date_specifier(&mut output, date, modified)?;
        } else {
            push_date_specifier(&mut output, date, specifier)?;
        }
    }
    Ok(output)
}

fn modified_date_specifier(modifier: char, specifier: Option<char>) -> Option<char> {
    let specifier = specifier?;
    match (modifier, specifier) {
        ('E', 'c' | 'C' | 'x' | 'X' | 'y' | 'Y') => Some(specifier),
        ('O', 'd' | 'e' | 'H' | 'I' | 'm' | 'M' | 'S' | 'u' | 'U' | 'V' | 'w' | 'W' | 'y') => {
            Some(specifier)
        }
        _ => None,
    }
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
        'U' => output.push_str(&format!("{:02}", week_number_sunday(date))),
        'W' => output.push_str(&format!("{:02}", week_number_monday(date))),
        'V' => output.push_str(&format!("{:02}", iso_week_year(date).1)),
        'G' => output.push_str(&format!("{:04}", iso_week_year(date).0)),
        'g' => output.push_str(&format!("{:02}", iso_week_year(date).0.rem_euclid(100))),
        'j' => output.push_str(&format!("{:03}", date.yday)),
        'u' => output.push_str(&iso_weekday(date).to_string()),
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
        'c' => {
            push_date_specifier(output, date, 'a')?;
            output.push(' ');
            push_date_specifier(output, date, 'b')?;
            output.push(' ');
            push_date_specifier(output, date, 'e')?;
            output.push(' ');
            push_date_specifier(output, date, 'T')?;
            output.push(' ');
            push_date_specifier(output, date, 'Y')?;
        }
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
        'x' => {
            push_date_specifier(output, date, 'm')?;
            output.push('/');
            push_date_specifier(output, date, 'd')?;
            output.push('/');
            push_date_specifier(output, date, 'y')?;
        }
        'X' => push_date_specifier(output, date, 'T')?,
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

fn week_number_sunday(date: UtcDateTime) -> i64 {
    let yday = date.yday - 1;
    let weekday = date.wday - 1;
    (yday + 7 - weekday) / 7
}

fn week_number_monday(date: UtcDateTime) -> i64 {
    let yday = date.yday - 1;
    let weekday = (date.wday + 5) % 7;
    (yday + 7 - weekday) / 7
}

fn iso_weekday(date: UtcDateTime) -> i64 {
    if date.wday == 1 { 7 } else { date.wday - 1 }
}

fn iso_week_year(date: UtcDateTime) -> (i64, i64) {
    let week = (date.yday - iso_weekday(date) + 10) / 7;
    if week < 1 {
        let year = date.year - 1;
        (year, weeks_in_iso_year(year))
    } else if week > weeks_in_iso_year(date.year) {
        (date.year + 1, 1)
    } else {
        (date.year, week)
    }
}

fn weeks_in_iso_year(year: i64) -> i64 {
    let jan_1_weekday = weekday_for_civil(year, 1, 1);
    if jan_1_weekday == 5 || (jan_1_weekday == 4 && is_leap_year(year)) {
        53
    } else {
        52
    }
}

fn weekday_for_civil(year: i64, month: i64, day: i64) -> i64 {
    (days_from_civil(year, month, day) + 4).rem_euclid(7) + 1
}

fn is_leap_year(year: i64) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
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
