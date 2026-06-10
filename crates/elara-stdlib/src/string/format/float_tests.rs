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
fn string_format_formats_float_precision() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%.2f:%.1e:%.1E:%.4g:%.4G:%.0g:%.0f");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::float(1.25),
            Value::float(12.5),
            Value::float(12.5),
            Value::float(12.5),
            Value::float(1_200_000.0),
            Value::float(12.5),
            Value::float(12.5),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"1.25:1.2e+01:1.2E+01:12.5:1.2E+06:1e+01:12".as_slice())
    );
}

#[test]
fn string_format_formats_float_width_and_flags() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%+8.2f:% 8.2f:%08.2f:%+08.2f:%-8.2f");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::float(1.25),
            Value::float(1.25),
            Value::float(1.25),
            Value::float(1.25),
            Value::float(1.25),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"   +1.25:    1.25:00001.25:+0001.25:1.25    ".as_slice())
    );
}

#[test]
fn string_format_formats_float_alternate_form() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%#.0f:%#.0e:%#.4g:%#.4G:%#8.4g:%#.0g");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::float(12.5),
            Value::float(12.5),
            Value::float(12.5),
            Value::float(1_200_000.0),
            Value::float(12.5),
            Value::float(12.5),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"12.:1.e+01:12.50:1.200E+06:   12.50:1.e+01".as_slice())
    );
}

#[test]
fn string_format_formats_basic_hex_float_conversion() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%a:%A:%a:%a:%a");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::float(12.5),
            Value::float(12.5),
            Value::float(0.0),
            Value::float(-0.0),
            Value::float(0.1),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"0x1.9p+3:0X1.9P+3:0x0p+0:-0x0p+0:0x1.999999999999ap-4".as_slice())
    );
}

#[test]
fn string_format_formats_hex_float_width_and_flags() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%14a:%+14a:%014a:%-14a:% 14a:%#a:%#A:%#a");

    let values = string_format(
        &mut runtime,
        &[
            format,
            Value::float(12.5),
            Value::float(12.5),
            Value::float(12.5),
            Value::float(12.5),
            Value::float(12.5),
            Value::float(8.0),
            Value::float(8.0),
            Value::float(0.0),
        ],
    )
    .expect("format should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(
            b"      0x1.9p+3:     +0x1.9p+3:0x0000001.9p+3:0x1.9p+3      :      0x1.9p+3:0x1.p+3:0X1.P+3:0x0.p+0"
                .as_slice()
        )
    );
}

#[test]
fn string_format_reports_hex_float_precision_gap() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%.3a");

    assert_eq!(
        string_format(&mut runtime, &[format, Value::float(12.5)])
            .expect_err("hex precision should still be explicit gap")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "string.format conversions are not supported yet".into(),
        }
    );
}

#[test]
fn string_format_reports_invalid_float_precision() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"%.123f");

    assert_eq!(
        string_format(&mut runtime, &[format, Value::float(1.0)])
            .expect_err("three-digit precision should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "invalid conversion specification".into(),
        }
    );
}
