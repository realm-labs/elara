//! `string.format` native implementation.

use elara_core::{LuaFloat, LuaInteger, Value};

use crate::{NativeError, NativeErrorKind, NativeRuntime, number::parse_standard_number};

use super::string_arg;

pub(super) fn string_format(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let format = string_arg(
        runtime,
        *args
            .first()
            .ok_or(NativeErrorKind::MissingArgument { index: 1 })?,
        1,
    )?
    .to_vec();
    let mut output = Vec::with_capacity(format.len());
    let mut index = 0;
    let mut arg_index = 1;
    while index < format.len() {
        if format[index] != b'%' {
            output.push(format[index]);
            index += 1;
        } else if format.get(index + 1) == Some(&b'%') {
            output.push(b'%');
            index += 2;
        } else if format.get(index + 1) == Some(&b's') {
            let value = next_format_arg(args, arg_index)?;
            output.extend_from_slice(&format_string_arg(runtime, value));
            arg_index += 1;
            index += 2;
        } else if let Some(spec @ (b'd' | b'i' | b'u' | b'o' | b'x' | b'X')) =
            format.get(index + 1).copied()
        {
            let value =
                integer_format_arg(runtime, next_format_arg(args, arg_index)?, arg_index + 1)?;
            output.extend_from_slice(format_integer_conversion(spec, value).as_bytes());
            arg_index += 1;
            index += 2;
        } else {
            return Err(NativeErrorKind::RuntimeError {
                message: "string.format conversions are not supported yet".into(),
            }
            .into());
        }
    }

    Ok(vec![runtime.intern_short_string(&output)?])
}

fn next_format_arg(args: &[Value], arg_index: usize) -> Result<Value, NativeError> {
    args.get(arg_index).copied().ok_or(
        NativeErrorKind::MissingArgument {
            index: arg_index + 1,
        }
        .into(),
    )
}

fn format_string_arg(runtime: &dyn NativeRuntime, value: Value) -> Vec<u8> {
    runtime
        .short_string_bytes(value)
        .map_or_else(|| tostring_bytes(value).into_bytes(), <[u8]>::to_vec)
}

fn integer_format_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<LuaInteger, NativeError> {
    if let Some(integer) = value.as_integer() {
        return Ok(integer);
    }
    if let Some(float) = value.as_float() {
        return floor_to_integer(float).ok_or_else(|| integer_type_error(index));
    }
    if let Some(bytes) = runtime.short_string_bytes(value)
        && let Some(number) = parse_standard_number(bytes)
    {
        return integer_format_arg(runtime, number, index);
    }
    Err(integer_type_error(index))
}

fn floor_to_integer(value: LuaFloat) -> Option<LuaInteger> {
    if !value.is_finite() {
        return None;
    }
    let value = value.floor();
    if value < LuaInteger::MIN as LuaFloat || value >= (LuaInteger::MAX as LuaFloat + 1.0) {
        return None;
    }
    Some(value as LuaInteger)
}

fn integer_type_error(index: usize) -> NativeError {
    NativeErrorKind::TypeError {
        index,
        expected: "integer",
    }
    .into()
}

fn format_integer_conversion(spec: u8, value: LuaInteger) -> String {
    match spec {
        b'd' | b'i' => value.to_string(),
        b'u' => (value as u64).to_string(),
        b'o' => format!("{:o}", value as u64),
        b'x' => format!("{:x}", value as u64),
        b'X' => format!("{:X}", value as u64),
        _ => unreachable!("caller filters integer conversion specifiers"),
    }
}

fn tostring_bytes(value: Value) -> String {
    if value.is_nil() {
        "nil".to_owned()
    } else if let Some(value) = value.as_bool() {
        value.to_string()
    } else if let Some(value) = value.as_integer() {
        value.to_string()
    } else if let Some(value) = value.as_float() {
        value.to_string()
    } else if let Some(index) = value.as_table_index() {
        format!("table: 0x{index:x}")
    } else if let Some(index) = value.as_closure_index() {
        format!("function: 0x{index:x}")
    } else if let Some(index) = value.as_native_function_index() {
        format!("function: 0x{index:x}")
    } else {
        "unknown: 0x0".to_owned()
    }
}

#[cfg(test)]
mod tests {
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
        let format = runtime.push_string(b"%f");

        assert_eq!(
            string_format(&mut runtime, &[format])
                .expect_err("conversion should be explicit gap")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "string.format conversions are not supported yet".into()
            }
        );
    }
}
