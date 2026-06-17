//! `string.packsize` native implementation.

use core::ffi::{c_int, c_long, c_short};
use core::mem::{align_of, size_of};

use elara_core::{LuaFloat, LuaInteger, Value};

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::string_arg;

const MAX_INTEGER_PACK_SIZE: usize = 16;

pub(super) fn string_packsize(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let format = string_arg(
        runtime,
        *args
            .first()
            .ok_or(NativeErrorKind::MissingArgument { index: 1 })?,
        1,
    )?;
    let mut parser = PackFormatParser::new(&format);
    let mut total_size = 0_usize;

    while parser.has_next() {
        let details = parser.next_details(total_size)?;
        if matches!(details.option, PackOption::String | PackOption::ZeroString) {
            return Err(NativeErrorKind::RuntimeError {
                message: "variable-length format".into(),
            }
            .into());
        }
        let size = details
            .size
            .checked_add(details.align_padding)
            .ok_or_else(packsize_too_large)?;
        total_size = total_size
            .checked_add(size)
            .ok_or_else(packsize_too_large)?;
        if total_size > LuaInteger::MAX as usize {
            return Err(packsize_too_large());
        }
    }

    Ok(vec![Value::integer(
        i64::try_from(total_size).expect("packsize is bounded by LuaInteger::MAX"),
    )])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackOption {
    Fixed,
    Char,
    String,
    ZeroString,
    PaddingAlign,
    NoOp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PackDetails {
    option: PackOption,
    size: usize,
    align_padding: usize,
}

struct PackFormatParser<'a> {
    format: &'a [u8],
    cursor: usize,
    max_align: usize,
}

impl<'a> PackFormatParser<'a> {
    fn new(format: &'a [u8]) -> Self {
        Self {
            format,
            cursor: 0,
            max_align: 1,
        }
    }

    fn has_next(&self) -> bool {
        self.cursor < self.format.len()
    }

    fn next_details(&mut self, total_size: usize) -> Result<PackDetails, NativeError> {
        let mut size = 0;
        let option = self.next_option(&mut size)?;
        let mut align = size;
        if option == PackOption::PaddingAlign {
            if !self.has_next() {
                return Err(invalid_next_option());
            }
            let next = self.next_option(&mut align)?;
            if next == PackOption::Char || align == 0 {
                return Err(invalid_next_option());
            }
        }

        let align_padding = if align <= 1 || option == PackOption::Char {
            0
        } else {
            let align = align.min(self.max_align);
            if !align.is_power_of_two() {
                return Err(NativeErrorKind::RuntimeError {
                    message: "format asks for alignment not power of 2".into(),
                }
                .into());
            }
            (align - (total_size & (align - 1))) & (align - 1)
        };

        Ok(PackDetails {
            option,
            size,
            align_padding,
        })
    }

    fn next_option(&mut self, size: &mut usize) -> Result<PackOption, NativeError> {
        let Some(option) = self.format.get(self.cursor).copied() else {
            return Ok(PackOption::NoOp);
        };
        self.cursor += 1;
        *size = 0;

        match option {
            b'b' | b'B' => {
                *size = size_of::<u8>();
                Ok(PackOption::Fixed)
            }
            b'h' | b'H' => {
                *size = size_of::<c_short>();
                Ok(PackOption::Fixed)
            }
            b'l' | b'L' => {
                *size = size_of::<c_long>();
                Ok(PackOption::Fixed)
            }
            b'j' | b'J' => {
                *size = size_of::<LuaInteger>();
                Ok(PackOption::Fixed)
            }
            b'T' => {
                *size = size_of::<usize>();
                Ok(PackOption::Fixed)
            }
            b'f' => {
                *size = size_of::<f32>();
                Ok(PackOption::Fixed)
            }
            b'n' => {
                *size = size_of::<LuaFloat>();
                Ok(PackOption::Fixed)
            }
            b'd' => {
                *size = size_of::<f64>();
                Ok(PackOption::Fixed)
            }
            b'i' | b'I' => {
                *size = self.integer_size(size_of::<c_int>())?;
                Ok(PackOption::Fixed)
            }
            b's' => {
                *size = self.integer_size(size_of::<usize>())?;
                Ok(PackOption::String)
            }
            b'c' => {
                *size = self
                    .number(None)?
                    .ok_or_else(|| NativeErrorKind::RuntimeError {
                        message: "missing size for format option 'c'".into(),
                    })?;
                Ok(PackOption::Char)
            }
            b'z' => Ok(PackOption::ZeroString),
            b'x' => {
                *size = 1;
                Ok(PackOption::Fixed)
            }
            b'X' => Ok(PackOption::PaddingAlign),
            b' ' => Ok(PackOption::NoOp),
            b'<' | b'>' | b'=' => Ok(PackOption::NoOp),
            b'!' => {
                self.max_align = self.integer_size(native_max_align())?;
                Ok(PackOption::NoOp)
            }
            option => Err(NativeErrorKind::RuntimeError {
                message: format!("invalid format option '{}'", char::from(option)).into(),
            }
            .into()),
        }
    }

    fn integer_size(&mut self, default: usize) -> Result<usize, NativeError> {
        let size = self.number(Some(default))?.unwrap_or(default);
        if !(1..=MAX_INTEGER_PACK_SIZE).contains(&size) {
            return Err(NativeErrorKind::RuntimeError {
                message: format!(
                    "integral size ({size}) out of limits [1,{MAX_INTEGER_PACK_SIZE}]"
                )
                .into(),
            }
            .into());
        }
        Ok(size)
    }

    fn number(&mut self, default: Option<usize>) -> Result<Option<usize>, NativeError> {
        let start = self.cursor;
        let mut value = 0_usize;
        while let Some(byte) = self.format.get(self.cursor).copied()
            && byte.is_ascii_digit()
        {
            let digit = usize::from(byte - b'0');
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(digit))
                .ok_or_else(packsize_too_large)?;
            self.cursor += 1;
        }
        if self.cursor == start {
            Ok(default)
        } else {
            Ok(Some(value))
        }
    }
}

fn native_max_align() -> usize {
    [
        align_of::<LuaFloat>(),
        align_of::<f64>(),
        align_of::<*const ()>(),
        align_of::<LuaInteger>(),
        align_of::<c_long>(),
    ]
    .into_iter()
    .max()
    .unwrap_or(1)
}

fn invalid_next_option() -> NativeError {
    NativeErrorKind::TypeError {
        index: 1,
        expected: "invalid next option for option 'X'",
    }
    .into()
}

fn packsize_too_large() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "format result too large".into(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::string_packsize;
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
}
