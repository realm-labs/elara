//! Table-related lowering for the simple compiler.

use elara_bytecode::Op;
use elara_core::{Diagnostic, Span, Value};
use elara_syntax::{Expr, ExprKind, TableField, TableFieldKind};

use super::SimpleCompiler;

impl SimpleCompiler {
    pub(super) fn compile_string_literal(&mut self, expr: &Expr<'_>, text: &str) -> u16 {
        let Some(bytes) = string_literal_bytes(text) else {
            self.diagnostics.push(
                Diagnostic::error("unsupported string literal in simple expression compiler")
                    .with_primary_span(expr.span()),
            );
            return self.alloc_register();
        };
        self.emit_string_constant(&bytes)
    }

    pub(super) fn compile_string_key(&mut self, _span: Span, text: &str) -> u16 {
        self.emit_string_constant(text.as_bytes())
    }

    pub(super) fn compile_table_constructor(&mut self, fields: &[TableField<'_>]) -> u16 {
        let table = self.alloc_register();
        let array_count = fields
            .iter()
            .filter(|field| matches!(field.kind(), TableFieldKind::Array(_)))
            .count();
        let hash_count = fields.len().saturating_sub(array_count);
        self.builder
            .emit_abc(Op::NewTable, table, array_count as u32, hash_count as u32);

        let mut array_index = 1_i64;
        for (field_index, field) in fields.iter().enumerate() {
            let is_last = field_index + 1 == fields.len();
            if let TableFieldKind::Array(value) = field.kind()
                && is_last
                && matches!(value.kind(), ExprKind::Call { .. } | ExprKind::Vararg)
            {
                let value_base = self.next_register;
                self.ensure_register_slot(value_base);
                match value.kind() {
                    ExprKind::Call { callee, args, .. } => {
                        self.compile_call_into_register(value, callee, args, value_base, 0);
                    }
                    ExprKind::Vararg => self.compile_vararg_into_register(value, value_base, 0),
                    _ => unreachable!("checked final open table field kind"),
                }
                self.builder.emit_abc(
                    Op::SetList,
                    table,
                    array_index as u32,
                    u32::from(value_base),
                );
                continue;
            }

            let (key, value) = match field.kind() {
                TableFieldKind::Array(value) => {
                    let key = self.emit_constant(Value::integer(array_index));
                    array_index += 1;
                    (key, self.compile_expr(value))
                }
                TableFieldKind::Named { name, value } => {
                    let key = self.emit_string_constant(name.as_bytes());
                    (key, self.compile_expr(value))
                }
                TableFieldKind::Keyed { key, value } => {
                    (self.compile_expr(key), self.compile_expr(value))
                }
            };
            self.builder
                .emit_abc(Op::SetTable, table, u32::from(key), u32::from(value));
        }

        table
    }

    pub(super) fn compile_table_get(&mut self, table: &Expr<'_>, key: &Expr<'_>) -> u16 {
        let table = self.compile_expr(table);
        let key = self.compile_expr(key);
        let result = self.alloc_register();
        self.builder
            .emit_abc(Op::GetTable, result, u32::from(table), u32::from(key));
        result
    }

    pub(super) fn compile_assignment_target(&mut self, target: &Expr<'_>, value: u16) {
        match target.kind() {
            ExprKind::Name(name) => self.compile_name_assignment(target, name, value),
            ExprKind::Index { table, key } => self.compile_table_assignment(table, key, value),
            _ => self.diagnostics.push(
                Diagnostic::error("unsupported assignment target").with_primary_span(target.span()),
            ),
        }
    }

    fn compile_table_assignment(&mut self, table: &Expr<'_>, key: &Expr<'_>, value: u16) {
        let table = self.compile_expr(table);
        let key = self.compile_expr(key);
        self.builder
            .emit_abc(Op::SetTable, table, u32::from(key), u32::from(value));
    }

    fn emit_string_constant(&mut self, bytes: &[u8]) -> u16 {
        let register = self.alloc_register();

        let constant = self.builder.add_string_constant(bytes);
        self.builder
            .emit_abx(Op::LoadString, register, u64::from(constant));
        register
    }
}

fn string_literal_bytes(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let quote = *bytes.first()?;
    if matches!(quote, b'\'' | b'"') {
        if bytes.last().copied() != Some(quote) {
            return None;
        }
        let inner = &bytes[1..bytes.len().checked_sub(1)?];
        return decode_short_string(inner);
    }

    if quote != b'[' {
        return None;
    }
    let mut level = 0;
    while bytes.get(level + 1).copied() == Some(b'=') {
        level += 1;
    }
    if bytes.get(level + 1).copied() != Some(b'[') {
        return None;
    }

    let open_end = level + 2;
    let close_start = bytes.len().checked_sub(level + 2)?;
    if close_start < open_end
        || bytes.get(close_start).copied() != Some(b']')
        || bytes.last().copied() != Some(b']')
        || bytes[close_start + 1..bytes.len() - 1]
            .iter()
            .any(|byte| *byte != b'=')
    {
        return None;
    }

    let mut inner = &bytes[open_end..close_start];
    if inner.starts_with(b"\r\n") {
        inner = &inner[2..];
    } else if inner.starts_with(b"\n") || inner.starts_with(b"\r") {
        inner = &inner[1..];
    }
    Some(inner.to_vec())
}

fn decode_short_string(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'\\' {
            decoded.push(byte);
            index += 1;
            continue;
        }

        index += 1;
        let escaped = *bytes.get(index)?;
        match escaped {
            b'a' => decoded.push(b'\x07'),
            b'b' => decoded.push(b'\x08'),
            b'f' => decoded.push(b'\x0c'),
            b'n' => decoded.push(b'\n'),
            b'r' => decoded.push(b'\r'),
            b't' => decoded.push(b'\t'),
            b'v' => decoded.push(b'\x0b'),
            b'\\' | b'"' | b'\'' => decoded.push(escaped),
            b'\n' | b'\r' => {
                decoded.push(b'\n');
                index = consume_newline(bytes, index);
                continue;
            }
            b'x' => {
                let high = hex_value(*bytes.get(index + 1)?)?;
                let low = hex_value(*bytes.get(index + 2)?)?;
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
            b'u' => {
                if bytes.get(index + 1).copied() != Some(b'{') {
                    return None;
                }
                let mut cursor = index + 2;
                let mut codepoint = 0_u32;
                let mut digit_count = 0;
                while let Some(value) = bytes.get(cursor).and_then(|byte| hex_value(*byte)) {
                    codepoint = codepoint.checked_mul(16)?.checked_add(u32::from(value))?;
                    digit_count += 1;
                    cursor += 1;
                }
                if digit_count == 0
                    || bytes.get(cursor).copied() != Some(b'}')
                    || codepoint > 0x7fff_ffff
                {
                    return None;
                }
                push_lua_utf8(&mut decoded, codepoint)?;
                index = cursor + 1;
                continue;
            }
            b'z' => {
                index += 1;
                while bytes.get(index).is_some_and(|byte| is_lua_space(*byte)) {
                    if matches!(bytes[index], b'\n' | b'\r') {
                        index = consume_newline(bytes, index);
                    } else {
                        index += 1;
                    }
                }
                continue;
            }
            b'0'..=b'9' => {
                let mut value = 0_u16;
                let mut digits = 0;
                while digits < 3 {
                    let Some(byte @ b'0'..=b'9') = bytes.get(index).copied() else {
                        break;
                    };
                    value = value * 10 + u16::from(byte - b'0');
                    index += 1;
                    digits += 1;
                }
                if value > u16::from(u8::MAX) {
                    return None;
                }
                decoded.push(value as u8);
                continue;
            }
            _ => return None,
        }
        index += 1;
    }
    Some(decoded)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn consume_newline(bytes: &[u8], index: usize) -> usize {
    let first = bytes[index];
    if (first == b'\r' && bytes.get(index + 1).copied() == Some(b'\n'))
        || (first == b'\n' && bytes.get(index + 1).copied() == Some(b'\r'))
    {
        index + 2
    } else {
        index + 1
    }
}

fn is_lua_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x0b')
}

fn push_lua_utf8(output: &mut Vec<u8>, codepoint: u32) -> Option<()> {
    let bytes = if codepoint <= 0x7f {
        1
    } else if codepoint <= 0x7ff {
        2
    } else if codepoint <= 0xffff {
        3
    } else if codepoint <= 0x1f_ffff {
        4
    } else if codepoint <= 0x3ff_ffff {
        5
    } else {
        6
    };
    let mut buffer = [0_u8; 6];
    let mut value = codepoint;
    for slot in (1..bytes).rev() {
        buffer[slot] = 0x80 | u8::try_from(value & 0x3f).ok()?;
        value >>= 6;
    }
    buffer[0] = match bytes {
        1 => u8::try_from(value).ok()?,
        2 => 0xc0 | u8::try_from(value).ok()?,
        3 => 0xe0 | u8::try_from(value).ok()?,
        4 => 0xf0 | u8::try_from(value).ok()?,
        5 => 0xf8 | u8::try_from(value).ok()?,
        6 => 0xfc | u8::try_from(value).ok()?,
        _ => return None,
    };
    output.extend_from_slice(&buffer[..bytes]);
    Some(())
}
