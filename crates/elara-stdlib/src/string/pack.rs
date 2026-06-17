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
    )?
    .to_vec();
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
    )?
    .to_vec();
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

pub(super) fn string_unpack(
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
    let data = string_arg(
        runtime,
        *args
            .get(1)
            .ok_or(NativeErrorKind::MissingArgument { index: 2 })?,
        2,
    )?
    .to_vec();
    let mut position = unpack_initial_position(args.get(2).copied(), data.len())?;
    let mut parser = PackFormatParser::new(&format);
    let mut values = Vec::new();

    while parser.has_next() {
        let details = parser.next_details(position)?;
        let needed = details
            .align_padding
            .checked_add(details.size)
            .ok_or_else(data_too_short)?;
        if needed > data.len().saturating_sub(position) {
            return Err(data_too_short());
        }
        position += details.align_padding;

        match details.option {
            PackOption::SignedInteger | PackOption::UnsignedInteger => {
                let integer = unpack_integer(
                    &data[position..position + details.size],
                    parser.is_little,
                    details.option == PackOption::SignedInteger,
                )?;
                values.push(Value::integer(integer));
            }
            PackOption::Float => {
                let bytes = endian_adjusted_bytes::<4>(
                    &data[position..position + details.size],
                    parser.is_little,
                );
                values.push(Value::float(f32::from_ne_bytes(bytes) as LuaFloat));
            }
            PackOption::Number | PackOption::Double => {
                let bytes = endian_adjusted_bytes::<8>(
                    &data[position..position + details.size],
                    parser.is_little,
                );
                values.push(Value::float(LuaFloat::from_ne_bytes(bytes)));
            }
            PackOption::Char => {
                values.push(runtime.intern_string(&data[position..position + details.size])?);
            }
            PackOption::String => {
                let len = unpack_integer(
                    &data[position..position + details.size],
                    parser.is_little,
                    false,
                )?;
                let len = usize::try_from(len).map_err(|_| data_too_short())?;
                let start = position + details.size;
                let end = start.checked_add(len).ok_or_else(data_too_short)?;
                if end > data.len() {
                    return Err(data_too_short());
                }
                values.push(runtime.intern_string(&data[start..end])?);
                position += len;
            }
            PackOption::ZeroString => {
                let Some(relative_end) = data[position..].iter().position(|byte| *byte == 0) else {
                    return Err(NativeErrorKind::RuntimeError {
                        message: "unfinished string for format 'z'".into(),
                    }
                    .into());
                };
                let end = position + relative_end;
                values.push(runtime.intern_string(&data[position..end])?);
                position = end + 1;
                continue;
            }
            PackOption::Padding | PackOption::PaddingAlign | PackOption::NoOp => {
                position += details.size;
                continue;
            }
        }
        position += details.size;
    }

    let next_position = i64::try_from(position + 1).map_err(|_| data_too_short())?;
    values.push(Value::integer(next_position));
    Ok(values)
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

fn unpack_integer(data: &[u8], is_little: bool, signed: bool) -> Result<LuaInteger, NativeError> {
    let limit = data.len().min(size_of::<LuaInteger>());
    let mut bits = 0_u64;
    for index in (0..limit).rev() {
        bits = (bits << BITS_PER_BYTE) | u64::from(order_byte(data, index, is_little));
    }

    if data.len() < size_of::<LuaInteger>() && signed {
        let sign_bit = 1_u64 << (data.len() * BITS_PER_BYTE - 1);
        bits = (bits ^ sign_bit).wrapping_sub(sign_bit);
    } else if data.len() > size_of::<LuaInteger>() {
        let sign_extend = if signed && (bits as LuaInteger) < 0 {
            u8::MAX
        } else {
            0
        };
        for index in limit..data.len() {
            if order_byte(data, index, is_little) != sign_extend {
                return Err(NativeErrorKind::RuntimeError {
                    message: format!("{}-byte integer does not fit into Lua Integer", data.len())
                        .into(),
                }
                .into());
            }
        }
    }

    Ok(bits as LuaInteger)
}

fn endian_adjusted_bytes<const N: usize>(data: &[u8], is_little: bool) -> [u8; N] {
    let mut bytes = [0; N];
    bytes.copy_from_slice(data);
    if is_little != cfg!(target_endian = "little") {
        bytes.reverse();
    }
    bytes
}

fn order_byte(data: &[u8], order_index: usize, is_little: bool) -> u8 {
    if is_little {
        data[order_index]
    } else {
        data[data.len() - 1 - order_index]
    }
}

fn unpack_initial_position(value: Option<Value>, len: usize) -> Result<usize, NativeError> {
    let position = match value {
        Some(value) if value.is_nil() => 1,
        Some(value) => value.as_integer().ok_or_else(|| {
            NativeError::from(NativeErrorKind::TypeError {
                index: 3,
                expected: "integer",
            })
        })?,
        None => 1,
    };
    let normalized = relative_unpack_position(position, len);
    if normalized == 0 || normalized > len.saturating_add(1) {
        return Err(NativeErrorKind::RuntimeError {
            message: "initial position out of string".into(),
        }
        .into());
    }
    Ok(normalized - 1)
}

fn relative_unpack_position(position: LuaInteger, len: usize) -> usize {
    let len = LuaInteger::try_from(len).expect("runtime string length fits LuaInteger");
    if position > 0 {
        usize::try_from(position).unwrap_or(usize::MAX)
    } else if position == 0 || position < -len {
        1
    } else {
        usize::try_from(len + position + 1).expect("relative position is positive")
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

fn data_too_short() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "data string too short".into(),
    }
    .into()
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod tests;
