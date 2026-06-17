//! `string.pack` and `string.packsize` native implementations.

use core::ffi::{c_int, c_long, c_short};
use core::mem::{align_of, size_of};

use elara_core::{LuaFloat, LuaInteger, Value};

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::string_arg;

const BITS_PER_BYTE: usize = 8;
const MAX_INTEGER_PACK_SIZE: usize = 16;

pub(super) fn string_pack(
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
    let mut output = Vec::new();
    let mut arg_index = 2;

    while parser.has_next() {
        let details = parser.next_details(output.len())?;
        output.extend(std::iter::repeat_n(0, details.align_padding));

        match details.option {
            PackOption::SignedInteger => {
                let integer = integer_arg(args, arg_index)?;
                pack_signed_integer(&mut output, integer, details.size, parser.is_little)?;
                arg_index += 1;
            }
            PackOption::UnsignedInteger => {
                let integer = integer_arg(args, arg_index)?;
                pack_unsigned_integer(&mut output, integer, details.size, parser.is_little)?;
                arg_index += 1;
            }
            PackOption::Float => {
                let float = number_arg(args, arg_index)?;
                pack_bytes(
                    &mut output,
                    &f32::to_ne_bytes(float as f32),
                    parser.is_little,
                );
                arg_index += 1;
            }
            PackOption::Number | PackOption::Double => {
                let float = number_arg(args, arg_index)?;
                pack_bytes(&mut output, &LuaFloat::to_ne_bytes(float), parser.is_little);
                arg_index += 1;
            }
            PackOption::Char => {
                let bytes = string_arg(
                    runtime,
                    *args
                        .get(arg_index - 1)
                        .ok_or(NativeErrorKind::MissingArgument { index: arg_index })?,
                    arg_index,
                )?;
                if bytes.len() > details.size {
                    return Err(NativeErrorKind::RuntimeError {
                        message: "string longer than given size".into(),
                    }
                    .into());
                }
                output.extend_from_slice(&bytes);
                output.extend(std::iter::repeat_n(0, details.size - bytes.len()));
                arg_index += 1;
            }
            PackOption::String => {
                let bytes = string_arg(
                    runtime,
                    *args
                        .get(arg_index - 1)
                        .ok_or(NativeErrorKind::MissingArgument { index: arg_index })?,
                    arg_index,
                )?;
                let len = LuaInteger::try_from(bytes.len()).map_err(|_| {
                    NativeErrorKind::RuntimeError {
                        message: "string length does not fit in pack format".into(),
                    }
                })?;
                pack_unsigned_integer(&mut output, len, details.size, parser.is_little)?;
                output.extend_from_slice(&bytes);
                arg_index += 1;
            }
            PackOption::ZeroString => {
                let bytes = string_arg(
                    runtime,
                    *args
                        .get(arg_index - 1)
                        .ok_or(NativeErrorKind::MissingArgument { index: arg_index })?,
                    arg_index,
                )?;
                if bytes.contains(&0) {
                    return Err(NativeErrorKind::RuntimeError {
                        message: "strings contains zeros".into(),
                    }
                    .into());
                }
                output.extend_from_slice(&bytes);
                output.push(0);
                arg_index += 1;
            }
            PackOption::Padding => output.push(0),
            PackOption::PaddingAlign | PackOption::NoOp => {}
        }
    }

    Ok(vec![runtime.intern_string(&output)?])
}

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
    SignedInteger,
    UnsignedInteger,
    Float,
    Number,
    Double,
    Char,
    String,
    ZeroString,
    Padding,
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
    is_little: bool,
}

impl<'a> PackFormatParser<'a> {
    fn new(format: &'a [u8]) -> Self {
        Self {
            format,
            cursor: 0,
            max_align: 1,
            is_little: cfg!(target_endian = "little"),
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
            b'b' => {
                *size = size_of::<u8>();
                Ok(PackOption::SignedInteger)
            }
            b'B' => {
                *size = size_of::<u8>();
                Ok(PackOption::UnsignedInteger)
            }
            b'h' => {
                *size = size_of::<c_short>();
                Ok(PackOption::SignedInteger)
            }
            b'H' => {
                *size = size_of::<c_short>();
                Ok(PackOption::UnsignedInteger)
            }
            b'l' => {
                *size = size_of::<c_long>();
                Ok(PackOption::SignedInteger)
            }
            b'L' => {
                *size = size_of::<c_long>();
                Ok(PackOption::UnsignedInteger)
            }
            b'j' => {
                *size = size_of::<LuaInteger>();
                Ok(PackOption::SignedInteger)
            }
            b'J' => {
                *size = size_of::<LuaInteger>();
                Ok(PackOption::UnsignedInteger)
            }
            b'T' => {
                *size = size_of::<usize>();
                Ok(PackOption::UnsignedInteger)
            }
            b'f' => {
                *size = size_of::<f32>();
                Ok(PackOption::Float)
            }
            b'n' => {
                *size = size_of::<LuaFloat>();
                Ok(PackOption::Number)
            }
            b'd' => {
                *size = size_of::<f64>();
                Ok(PackOption::Double)
            }
            b'i' => {
                *size = self.integer_size(size_of::<c_int>())?;
                Ok(PackOption::SignedInteger)
            }
            b'I' => {
                *size = self.integer_size(size_of::<c_int>())?;
                Ok(PackOption::UnsignedInteger)
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
                Ok(PackOption::Padding)
            }
            b'X' => Ok(PackOption::PaddingAlign),
            b' ' => Ok(PackOption::NoOp),
            b'<' => {
                self.is_little = true;
                Ok(PackOption::NoOp)
            }
            b'>' => {
                self.is_little = false;
                Ok(PackOption::NoOp)
            }
            b'=' => {
                self.is_little = cfg!(target_endian = "little");
                Ok(PackOption::NoOp)
            }
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

fn pack_signed_integer(
    output: &mut Vec<u8>,
    integer: LuaInteger,
    size: usize,
    is_little: bool,
) -> Result<(), NativeError> {
    if size < size_of::<LuaInteger>() {
        let bits = size * BITS_PER_BYTE;
        let limit = 1_i128 << (bits - 1);
        let integer = i128::from(integer);
        if !(-limit..limit).contains(&integer) {
            return Err(NativeErrorKind::RuntimeError {
                message: "integer overflow".into(),
            }
            .into());
        }
    }
    pack_integer_bits(output, integer as u64, size, is_little, integer < 0);
    Ok(())
}

fn pack_unsigned_integer(
    output: &mut Vec<u8>,
    integer: LuaInteger,
    size: usize,
    is_little: bool,
) -> Result<(), NativeError> {
    if size < size_of::<LuaInteger>() {
        let bits = size * BITS_PER_BYTE;
        if (integer as u64) >= (1_u64 << bits) {
            return Err(NativeErrorKind::RuntimeError {
                message: "unsigned overflow".into(),
            }
            .into());
        }
    }
    pack_integer_bits(output, integer as u64, size, is_little, false);
    Ok(())
}

fn pack_integer_bits(
    output: &mut Vec<u8>,
    mut bits: u64,
    size: usize,
    is_little: bool,
    negative: bool,
) {
    let mut bytes = vec![0; size];
    for index in 0..size {
        let byte = if index < size_of::<LuaInteger>() {
            let byte = bits as u8;
            bits >>= BITS_PER_BYTE;
            byte
        } else if negative {
            u8::MAX
        } else {
            0
        };
        let target = if is_little { index } else { size - 1 - index };
        bytes[target] = byte;
    }
    output.extend_from_slice(&bytes);
}

fn pack_bytes(output: &mut Vec<u8>, bytes: &[u8], is_little: bool) {
    if is_little == cfg!(target_endian = "little") {
        output.extend_from_slice(bytes);
    } else {
        output.extend(bytes.iter().rev().copied());
    }
}

fn integer_arg(args: &[Value], index: usize) -> Result<LuaInteger, NativeError> {
    args.get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?
        .as_integer()
        .ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "integer",
            }
            .into(),
        )
}

fn number_arg(args: &[Value], index: usize) -> Result<LuaFloat, NativeError> {
    args.get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?
        .to_float()
        .ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "number",
            }
            .into(),
        )
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
#[path = "pack_tests.rs"]
mod tests;
