//! Reusable fuzz target entry points.
//!
//! These functions are deterministic and side-effect free so they can be
//! called from unit tests now and wired into a fuzzing engine later.

use elara_bytecode::{Instr, Op, ProtoBuilder, verify_proto};
use elara_compiler::compile_simple_chunk;
use elara_core::{SourceId, Table, Value};
use elara_syntax::{lex, parse_chunk};

const MAX_BYTECODE_INSTRUCTIONS: usize = 64;
const MAX_TABLE_OPERATIONS: usize = 256;

/// Exercises the lexer, parser, and simple compiler with arbitrary bytes.
pub fn fuzz_parser_input(bytes: &[u8]) {
    let source = String::from_utf8_lossy(bytes);
    let source_id = source_id_from_bytes(bytes);

    let _ = lex(source_id, &source);
    let _ = parse_chunk(source_id, &source);
    let _ = compile_simple_chunk(source_id, &source);
}

/// Builds a small arbitrary prototype and feeds it to the bytecode verifier.
pub fn fuzz_bytecode_verifier(bytes: &[u8]) {
    let max_stack = u16::from(bytes.first().copied().unwrap_or(1) % 8);
    let params = bytes.get(1).copied().unwrap_or(0) % 4;
    let is_vararg = bytes.get(2).is_some_and(|byte| byte & 1 != 0);
    let constant_count = usize::from(bytes.get(3).copied().unwrap_or(0) % 4);
    let string_count = usize::from(bytes.get(4).copied().unwrap_or(0) % 4);

    let mut builder = ProtoBuilder::new().with_signature(max_stack, params, is_vararg);

    for index in 0..constant_count {
        let value = bytes
            .get(5 + index)
            .map_or(Value::nil(), |byte| Value::integer(i64::from(*byte)));
        builder.add_constant(value);
    }

    for index in 0..string_count {
        let byte = bytes.get(9 + index).copied().unwrap_or_default();
        builder.add_string_constant([byte]);
    }

    let instruction_bytes = bytes.get(13..).unwrap_or(&[]);
    for chunk in instruction_bytes.chunks(6).take(MAX_BYTECODE_INSTRUCTIONS) {
        let op = Op::from_byte(chunk.first().copied().unwrap_or(Op::Return as u8) % 53)
            .unwrap_or(Op::Return);
        let a = u16::from(chunk.get(1).copied().unwrap_or_default());
        let b = u32::from(chunk.get(2).copied().unwrap_or_default())
            | (u32::from(chunk.get(3).copied().unwrap_or_default()) << 8);
        let c = u32::from(chunk.get(4).copied().unwrap_or_default())
            | (u32::from(chunk.get(5).copied().unwrap_or_default()) << 8);

        builder.emit(Instr::abc(op, a, b, c));
    }

    let proto = builder.finish();
    let _ = verify_proto(&proto);
}

/// Exercises raw table reads, writes, clears, and invalid keys.
pub fn fuzz_table_operations(bytes: &[u8]) {
    let mut table = Table::new();

    for chunk in bytes.chunks(5).take(MAX_TABLE_OPERATIONS) {
        let opcode = chunk.first().copied().unwrap_or_default() % 4;
        let key = table_key_from_bytes(chunk.get(1).copied().unwrap_or_default());
        let value = table_value_from_bytes(
            chunk.get(2).copied().unwrap_or_default(),
            chunk.get(3).copied().unwrap_or_default(),
        );
        let integer_key = bounded_integer(chunk.get(4).copied().unwrap_or_default());

        match opcode {
            0 => {
                let _ = table.raw_set_value(key, value);
            }
            1 => {
                let _ = table.raw_get_value(key);
            }
            2 => {
                let _ = table.raw_set_integer(integer_key, value);
            }
            _ => {
                let _ = table.raw_get_integer(integer_key);
            }
        }
    }
}

fn source_id_from_bytes(bytes: &[u8]) -> SourceId {
    let mut raw = 0_u32;
    for (shift, byte) in bytes.iter().copied().take(4).enumerate() {
        raw |= u32::from(byte) << (shift * 8);
    }
    SourceId::new(raw)
}

fn table_key_from_bytes(byte: u8) -> Value {
    match byte % 5 {
        0 => Value::nil(),
        1 => Value::boolean(byte & 0x80 != 0),
        2 => Value::integer(bounded_integer(byte)),
        3 if byte == u8::MAX => Value::float(f64::NAN),
        3 => Value::float(f64::from(byte) / 3.0),
        _ => Value::table_index(u32::from(byte)),
    }
}

fn table_value_from_bytes(tag: u8, payload: u8) -> Value {
    match tag % 6 {
        0 => Value::nil(),
        1 => Value::boolean(payload & 1 != 0),
        2 => Value::integer(i64::from(i8::from_ne_bytes([payload]))),
        3 if payload == u8::MAX => Value::float(f64::NAN),
        3 => Value::float(f64::from(payload) / 8.0),
        4 => Value::closure_index(u32::from(payload)),
        _ => Value::thread_index(u32::from(payload)),
    }
}

fn bounded_integer(byte: u8) -> i64 {
    i64::from(byte % 33) - 16
}

#[cfg(test)]
mod tests {
    use super::{fuzz_bytecode_verifier, fuzz_parser_input, fuzz_table_operations};

    #[test]
    fn fuzz_parser_target_accepts_arbitrary_bytes() {
        fuzz_parser_input(b"return 1 + 2");
        fuzz_parser_input(&[0, 159, 146, 150, b'r', b'e', b't', b'u', b'r', b'n']);
    }

    #[test]
    fn fuzz_bytecode_verifier_target_accepts_arbitrary_bytes() {
        fuzz_bytecode_verifier(&[]);
        fuzz_bytecode_verifier(&[0, 1, 1, 2, 2, 42, 43, 1, 2, 3, 4, 5, 6, 52, 255, 0, 1]);
    }

    #[test]
    fn fuzz_table_operations_target_accepts_arbitrary_bytes() {
        fuzz_table_operations(&[]);
        fuzz_table_operations(&[0, 2, 3, 4, 5, 1, 255, 3, 255, 31, 2, 4, 0, 9, 32]);
    }
}
