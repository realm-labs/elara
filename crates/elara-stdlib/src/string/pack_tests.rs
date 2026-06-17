use elara_core::Value;

use super::{string_pack, string_packsize, string_unpack};
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
fn string_pack_writes_integer_character_and_padding_formats() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"<bBI2i2c4xz");
    let text = runtime.push_string(b"hi");
    let zstr = runtime.push_string(b"ok");

    let values = string_pack(
        &mut runtime,
        &[
            format,
            Value::integer(-1),
            Value::integer(255),
            Value::integer(0x1234),
            Value::integer(-2),
            text,
            zstr,
        ],
    )
    .expect("pack should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(
            &[
                255, 255, 0x34, 0x12, 0xfe, 0xff, b'h', b'i', 0, 0, 0, b'o', b'k', 0
            ][..]
        )
    );
}

#[test]
fn string_pack_honors_big_endian_and_alignment_formats() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b">!4bI4Xdb");

    let values = string_pack(
        &mut runtime,
        &[
            format,
            Value::integer(1),
            Value::integer(0x01020304),
            Value::integer(0),
        ],
    )
    .expect("pack should pass");

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(&[1, 0, 0, 0, 1, 2, 3, 4, 0][..])
    );
}

#[test]
fn string_pack_reports_value_errors() {
    let mut runtime = TestRuntime::default();
    let overflow_signed = runtime.push_string(b"b");
    let overflow_unsigned = runtime.push_string(b"B");
    let long_char = runtime.push_string(b"c1");
    let text = runtime.push_string(b"ab");
    let zero_string = runtime.push_string(b"z");
    let zero_text = runtime.push_string(b"a\0b");

    assert_eq!(
        string_pack(&mut runtime, &[overflow_signed, Value::integer(128)])
            .expect_err("signed overflow should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "integer overflow".into()
        }
    );
    assert_eq!(
        string_pack(&mut runtime, &[overflow_unsigned, Value::integer(256)])
            .expect_err("unsigned overflow should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "unsigned overflow".into()
        }
    );
    assert_eq!(
        string_pack(&mut runtime, &[long_char, text])
            .expect_err("long c string should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "string longer than given size".into()
        }
    );
    assert_eq!(
        string_pack(&mut runtime, &[zero_string, zero_text])
            .expect_err("zero string with embedded zero should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "strings contains zeros".into()
        }
    );
}

#[test]
fn string_unpack_reads_integer_character_and_padding_formats() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"<bBI2i2c4xz");
    let data = runtime.push_string(&[
        255, 255, 0x34, 0x12, 0xfe, 0xff, b'h', b'i', 0, 0, 0, b'o', b'k', 0,
    ]);

    let values = string_unpack(&mut runtime, &[format, data]).expect("unpack should pass");

    assert_eq!(values[0], Value::integer(-1));
    assert_eq!(values[1], Value::integer(255));
    assert_eq!(values[2], Value::integer(0x1234));
    assert_eq!(values[3], Value::integer(-2));
    assert_eq!(runtime.short_string_bytes(values[4]), Some(&b"hi\0\0"[..]));
    assert_eq!(runtime.short_string_bytes(values[5]), Some(&b"ok"[..]));
    assert_eq!(values[6], Value::integer(15));
}

#[test]
fn string_unpack_honors_big_endian_alignment() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b">!4bI4Xdb");
    let data = runtime.push_string(&[1, 0, 0, 0, 1, 2, 3, 4, 0]);

    let values = string_unpack(&mut runtime, &[format, data]).expect("unpack should pass");

    assert_eq!(
        values,
        vec![
            Value::integer(1),
            Value::integer(0x01020304),
            Value::integer(0),
            Value::integer(10)
        ]
    );
}

#[test]
fn string_unpack_honors_initial_position() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"B");
    let data = runtime.push_string(&[9, 42]);

    let values = string_unpack(&mut runtime, &[format, data, Value::integer(2)])
        .expect("unpack should pass");

    assert_eq!(values, vec![Value::integer(42), Value::integer(3)]);
}

#[test]
fn string_unpack_reads_length_prefixed_and_zero_strings() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"<s1z");
    let data = runtime.push_string(&[3, b'a', b'b', b'c', b'o', b'k', 0]);

    let values = string_unpack(&mut runtime, &[format, data]).expect("unpack should pass");

    assert_eq!(runtime.short_string_bytes(values[0]), Some(&b"abc"[..]));
    assert_eq!(runtime.short_string_bytes(values[1]), Some(&b"ok"[..]));
    assert_eq!(values[2], Value::integer(8));
}

#[test]
fn string_unpack_reports_data_errors() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"I4");
    let short = runtime.push_string(&[1, 2]);
    let z_format = runtime.push_string(b"z");
    let unfinished = runtime.push_string(b"abc");
    let valid = runtime.push_string(&[0, 0, 0, 0]);

    assert_eq!(
        string_unpack(&mut runtime, &[format, short])
            .expect_err("short data should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "data string too short".into()
        }
    );
    assert_eq!(
        string_unpack(&mut runtime, &[z_format, unfinished])
            .expect_err("unfinished zero string should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "unfinished string for format 'z'".into()
        }
    );
    assert_eq!(
        string_unpack(&mut runtime, &[format, valid, Value::integer(6)])
            .expect_err("out-of-range position should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "initial position out of string".into()
        }
    );
}

#[test]
fn string_packsize_counts_fixed_format_sizes() {
    let mut runtime = TestRuntime::default();
    let format = runtime.push_string(b"bBhHjJTfdnxi1I2c3");

    assert_eq!(
        string_packsize(&mut runtime, &[format]).expect("packsize should pass"),
        vec![Value::integer(57)]
    );
}

#[test]
fn string_packsize_honors_alignment_options() {
    let mut runtime = TestRuntime::default();
    let aligned = runtime.push_string(b"!4bI4Xdb");
    let unaligned = runtime.push_string(b"!1bI4Xdb");

    assert_eq!(
        string_packsize(&mut runtime, &[aligned]).expect("aligned packsize should pass"),
        vec![Value::integer(9)]
    );
    assert_eq!(
        string_packsize(&mut runtime, &[unaligned]).expect("unaligned packsize should pass"),
        vec![Value::integer(6)]
    );
}

#[test]
fn string_packsize_rejects_variable_length_formats() {
    let mut runtime = TestRuntime::default();
    let string_format = runtime.push_string(b"s");
    let zero_string_format = runtime.push_string(b"z");

    for format in [string_format, zero_string_format] {
        assert_eq!(
            string_packsize(&mut runtime, &[format])
                .expect_err("variable-length format should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "variable-length format".into()
            }
        );
    }
}

#[test]
fn string_packsize_reports_invalid_formats() {
    let mut runtime = TestRuntime::default();
    let missing_char_size = runtime.push_string(b"c");
    let invalid_align = runtime.push_string(b"!3i");
    let invalid_size = runtime.push_string(b"i17");
    let invalid_option = runtime.push_string(b"Q");
    let invalid_next = runtime.push_string(b"Xc1");

    assert_eq!(
        string_packsize(&mut runtime, &[missing_char_size])
            .expect_err("missing c size should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "missing size for format option 'c'".into()
        }
    );
    assert_eq!(
        string_packsize(&mut runtime, &[invalid_align])
            .expect_err("invalid alignment should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "format asks for alignment not power of 2".into()
        }
    );
    assert_eq!(
        string_packsize(&mut runtime, &[invalid_size])
            .expect_err("invalid integer size should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "integral size (17) out of limits [1,16]".into()
        }
    );
    assert_eq!(
        string_packsize(&mut runtime, &[invalid_option])
            .expect_err("invalid option should fail")
            .kind(),
        &NativeErrorKind::RuntimeError {
            message: "invalid format option 'Q'".into()
        }
    );
    assert_eq!(
        string_packsize(&mut runtime, &[invalid_next])
            .expect_err("invalid X next option should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "invalid next option for option 'X'",
        }
    );
}

#[test]
fn string_packsize_reports_argument_errors() {
    let mut runtime = TestRuntime::default();

    assert_eq!(
        string_packsize(&mut runtime, &[])
            .expect_err("missing format should fail")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        string_packsize(&mut runtime, &[Value::boolean(false)])
            .expect_err("non-string format should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "string"
        }
    );
}
