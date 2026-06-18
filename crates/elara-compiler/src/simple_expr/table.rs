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
        self.emit_string_constant(bytes)
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

fn string_literal_bytes(text: &str) -> Option<&[u8]> {
    let bytes = text.as_bytes();
    let quote = *bytes.first()?;
    if !matches!(quote, b'\'' | b'"') || bytes.last().copied() != Some(quote) {
        return None;
    }
    let inner = &bytes[1..bytes.len().checked_sub(1)?];
    if inner.contains(&b'\\') {
        return None;
    }
    Some(inner)
}
