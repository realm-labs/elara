use elara_core::Value;

use super::string_format;
use crate::{NativeError, NativeErrorKind, NativeRuntime};

#[derive(Default)]
struct TestRuntime {
    strings: Vec<Box<[u8]>>,
}

impl TestRuntime {
    fn push_string(&mut self, bytes: &[u8]) -> Value {
        let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
        self.strings.push(bytes.into());
        Value::table_index(index)
    }
}

impl NativeRuntime for TestRuntime {
    fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
        Ok(self.push_string(bytes))
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        let index = value.as_table_index()? as usize;
        self.strings.get(index).map(Box::as_ref)
    }
}

#[test]
fn string_format_returns_literal_format_without_conversions() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"hello");

    let values = string_format(&mut runtime, &[format]).expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"hello".as_slice())
    );
}

#[test]
fn string_format_unescapes_double_percent() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"a%%b%%%%");

    let values = string_format(&mut runtime, &[format]).expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"a%b%%".as_slice())
    );
}

#[test]
fn string_format_formats_basic_string_conversions() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%s:%s:%s");
    let label = runtime.push_string(b"value");

    let values = string_format(
        &mut runtime,
        &[format, label, Value::integer(7), Value::boolean(true)],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"value:7:true".as_slice())
    );
}

#[test]
fn string_format_formats_modified_string_conversions() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%5s:%-5s:%.3s:%5.2s");
    let text = runtime.push_string(b"abcd");

    let values =
        string_format(&mut runtime, &[format, text, text, text, text]).expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b" abcd:abcd :abc:   ab".as_slice())
    );
}

#[test]
fn string_format_rejects_zero_bytes_in_modified_string_conversion() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%5s");
    let text = runtime.push_string(b"a\0b");

    assert_eq!(
        string_format(&mut runtime, &[format, text])
            .expect_err("modified string conversion should reject zeros")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "string contains zeros".into(),
        }
    );
}

#[test]
fn string_format_reports_invalid_string_conversion_specification() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%05s");
    let text = runtime.push_string(b"ab");

    assert_eq!(
        string_format(&mut runtime, &[format, text])
            .expect_err("invalid string conversion should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "invalid conversion specification".into(),
        }
    );
}

#[test]
fn string_format_formats_basic_integer_conversions() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%d:%i:%d:%i:%u:%o:%x:%X");
    let numeric_string = runtime.push_string(b"12.9");
    let negative_string = runtime.push_string(b"-2.1");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::integer(7),
            Value::float(8.9),
            numeric_string,
            negative_string,
            Value::integer(7),
            Value::integer(8),
            Value::integer(255),
            Value::integer(255),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"7:8:12:-3:7:10:ff:FF".as_slice())
    );
}

#[test]
fn string_format_formats_integer_width() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%5d:%3i:%2d:%3u:%4o:%4x:%4X:%-5d:%-4x:%05d:%05i:%04x:%-04d");
    let numeric_string = runtime.push_string(b"12.9");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::integer(7),
            Value::integer(-7),
            numeric_string,
            Value::integer(7),
            Value::integer(8),
            Value::integer(255),
            Value::integer(255),
            Value::integer(7),
            Value::integer(255),
            Value::integer(7),
            Value::integer(-7),
            Value::integer(255),
            Value::integer(7),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"    7: -7:12:  7:  10:  ff:  FF:7    :ff  :00007:-0007:00ff:7   ".as_slice())
    );
}

#[test]
fn string_format_reports_invalid_integer_width() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%123x");

    assert_eq!(
        string_format(&mut runtime, &[format, Value::integer(7)])
            .expect_err("three-digit width should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "invalid conversion specification".into(),
        }
    );
}

#[test]
fn string_format_formats_unsigned_integer_bits() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%u:%x");

    let values = string_format(
        &mut runtime,
        &[format, Value::integer(-1), Value::integer(-1)],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(format!("{}:{:x}", u64::MAX, u64::MAX).as_bytes())
    );
}

#[test]
fn string_format_formats_basic_character_conversions() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%c%c");

    let values = string_format(
        &mut runtime,
        &[format, Value::integer(65), Value::integer(256)],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"A\0".as_slice())
    );
}

#[test]
fn string_format_formats_quoted_strings() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%q");
    let value = runtime.push_string(b"a\"\\\n");

    let values = string_format(&mut runtime, &[format, value]).expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"\"a\\\"\\\\\\\n\"".as_slice())
    );
}

#[test]
fn string_format_pads_quoted_control_bytes_before_digits() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%q");
    let value = runtime.push_string(b"\x012");

    let values = string_format(&mut runtime, &[format, value]).expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"\"\\0012\"".as_slice())
    );
}

#[test]
fn string_format_formats_quoted_scalar_literals() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%q:%q:%q:%q");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::nil(),
            Value::boolean(true),
            Value::integer(-7),
            Value::float(1.5),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"nil:true:-7:1.5".as_slice())
    );
}

#[test]
fn string_format_formats_basic_float_conversion() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%f:%f:%f:%e:%E:%g:%g:%G");
    let numeric_string = runtime.push_string(b"2.25");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::integer(7),
            Value::float(1.5),
            numeric_string,
            Value::float(12.5),
            Value::float(12.5),
            Value::float(12.5),
            Value::float(0.0000125),
            Value::float(1_200_000.0),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(
            b"7.000000:1.500000:2.250000:1.250000e+01:1.250000E+01:12.5:1.25e-05:1.2E+06"
                .as_slice()
        )
    );
}

#[test]
fn string_format_formats_basic_pointer_conversion() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%p:%p:%p");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::table_index(0),
            Value::closure_index(3),
            Value::native_function_index(4),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"0x1:0x4:0x5".as_slice())
    );
}

#[test]
fn string_format_formats_null_pointer_for_scalar_values() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%p:%p:%p");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::nil(),
            Value::boolean(false),
            Value::integer(7),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"(null):(null):(null)".as_slice())
    );
}

#[test]
fn string_format_reports_non_literal_quoted_argument() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%q");

    assert_eq!(
        string_format(&mut runtime, &[format, Value::closure_index(0)])
            .expect_err("non-literal value should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "literal",
        }
    );
}

#[test]
fn string_format_reports_missing_float_conversion_argument() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%f");

    assert_eq!(
        string_format(&mut runtime, &[format])
            .expect_err("missing conversion argument should fail")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
}

#[test]
fn string_format_reports_non_number_float_conversion_argument() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%f");

    assert_eq!(
        string_format(&mut runtime, &[format, Value::boolean(true)])
            .expect_err("non-number conversion argument should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "number",
        }
    );
}

#[test]
fn string_format_reports_missing_string_conversion_argument() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%s");

    assert_eq!(
        string_format(&mut runtime, &[format])
            .expect_err("missing conversion argument should fail")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
}

#[test]
fn string_format_reports_missing_integer_conversion_argument() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%d");

    assert_eq!(
        string_format(&mut runtime, &[format])
            .expect_err("missing conversion argument should fail")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
}

#[test]
fn string_format_reports_non_integer_conversion_argument() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%d");

    assert_eq!(
        string_format(&mut runtime, &[format, Value::boolean(true)])
            .expect_err("non-integer conversion argument should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        }
    );
}

#[test]
fn string_format_reports_conversion_gap() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%a");

    assert_eq!(
        string_format(&mut runtime, &[format])
            .expect_err("conversion should be explicit gap")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "string.format conversions are not supported yet".into()
        }
    );
}
