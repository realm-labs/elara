//! Simple expression bytecode lowering.

use elara_bytecode::{Op, Proto, ProtoBuilder, verify_proto};
use elara_core::{Diagnostic, SourceId, Value};
use elara_syntax::{BinaryOp, Expr, ExprKind, StmtKind, UnaryOp, parse_chunk};

/// Result of compiling a chunk.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileResult {
    /// Generated prototype when compilation succeeds.
    pub proto: Option<Proto>,
    /// Diagnostics from parsing or lowering.
    pub diagnostics: Vec<Diagnostic>,
}

/// Compiles a chunk containing simple return expressions.
#[must_use]
pub fn compile_simple_chunk(source: SourceId, input: &str) -> CompileResult {
    let parsed = parse_chunk(source, input);
    if !parsed.diagnostics.is_empty() {
        return CompileResult {
            proto: None,
            diagnostics: parsed.diagnostics,
        };
    }

    let mut compiler = SimpleCompiler {
        builder: ProtoBuilder::new(),
        diagnostics: Vec::new(),
        next_register: 0,
        max_register: 0,
    };
    compiler.compile_block(parsed.block.statements());
    compiler.finish()
}

struct SimpleCompiler {
    builder: ProtoBuilder,
    diagnostics: Vec<Diagnostic>,
    next_register: u16,
    max_register: u16,
}

impl SimpleCompiler {
    fn compile_block(&mut self, statements: &[elara_syntax::Stmt<'_>]) {
        for statement in statements {
            match statement.kind() {
                StmtKind::Return(values) => self.compile_return(values),
                _ => self.diagnostics.push(
                    Diagnostic::error("unsupported statement in simple expression compiler")
                        .with_primary_span(statement.span()),
                ),
            }
        }
    }

    fn compile_return(&mut self, values: &[Expr<'_>]) {
        let start = values.first().map_or(0, |_| self.next_register);
        for value in values {
            self.compile_expr(value);
        }
        self.builder
            .emit_abc(Op::Return, start, values.len() as u32, 0);
    }

    fn compile_expr(&mut self, expr: &Expr<'_>) -> u16 {
        match expr.kind() {
            ExprKind::Nil => {
                let register = self.alloc_register();
                self.builder.emit_abc(Op::LoadNil, register, 0, 0);
                register
            }
            ExprKind::Bool(value) => {
                let register = self.alloc_register();
                self.builder
                    .emit_abc(Op::LoadBool, register, u32::from(*value), 0);
                register
            }
            ExprKind::Integer(text) => self.compile_integer(expr, text),
            ExprKind::Float(text) => self.compile_float(expr, text),
            ExprKind::Grouped(expr) => self.compile_expr(expr),
            ExprKind::Unary { op, expr } => self.compile_unary(expr, *op),
            ExprKind::Binary { op, left, right } => self.compile_binary(expr, *op, left, right),
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("unsupported expression in simple expression compiler")
                        .with_primary_span(expr.span()),
                );
                self.alloc_register()
            }
        }
    }

    fn compile_integer(&mut self, expr: &Expr<'_>, text: &str) -> u16 {
        match text.parse::<i64>() {
            Ok(value) => self.emit_constant(Value::integer(value)),
            Err(_) => {
                self.diagnostics.push(
                    Diagnostic::error("invalid integer literal").with_primary_span(expr.span()),
                );
                self.alloc_register()
            }
        }
    }

    fn compile_float(&mut self, expr: &Expr<'_>, text: &str) -> u16 {
        match text.parse::<f64>() {
            Ok(value) => self.emit_constant(Value::float(value)),
            Err(_) => {
                self.diagnostics.push(
                    Diagnostic::error("invalid float literal").with_primary_span(expr.span()),
                );
                self.alloc_register()
            }
        }
    }

    fn emit_constant(&mut self, value: Value) -> u16 {
        let register = self.alloc_register();
        let constant = self.builder.add_constant(value);
        self.builder
            .emit_abx(Op::LoadK, register, u64::from(constant));
        register
    }

    fn compile_unary(&mut self, expr: &Expr<'_>, op: UnaryOp) -> u16 {
        let value = self.compile_expr(expr);
        let bytecode_op = match op {
            UnaryOp::Neg => Op::Unm,
            UnaryOp::BitNot => Op::BNot,
            UnaryOp::Len => Op::Len,
            UnaryOp::Not => {
                self.diagnostics.push(
                    Diagnostic::error("unsupported unary operator in simple expression compiler")
                        .with_primary_span(expr.span()),
                );
                return value;
            }
        };
        self.builder.emit_abc(bytecode_op, value, value.into(), 0);
        value
    }

    fn compile_binary(
        &mut self,
        expr: &Expr<'_>,
        op: BinaryOp,
        left: &Expr<'_>,
        right: &Expr<'_>,
    ) -> u16 {
        let left_register = self.compile_expr(left);
        let right_register = self.compile_expr(right);
        let bytecode_op = match op {
            BinaryOp::Add => Op::Add,
            BinaryOp::Sub => Op::Sub,
            BinaryOp::Mul => Op::Mul,
            BinaryOp::Div => Op::Div,
            BinaryOp::FloorDiv => Op::IDiv,
            BinaryOp::Mod => Op::Mod,
            BinaryOp::Pow => Op::Pow,
            BinaryOp::BitAnd => Op::BAnd,
            BinaryOp::BitOr => Op::BOr,
            BinaryOp::BitXor => Op::BXor,
            BinaryOp::ShiftLeft => Op::Shl,
            BinaryOp::ShiftRight => Op::Shr,
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("unsupported binary operator in simple expression compiler")
                        .with_primary_span(expr.span()),
                );
                return left_register;
            }
        };
        self.builder.emit_abc(
            bytecode_op,
            left_register,
            u32::from(left_register),
            u32::from(right_register),
        );
        left_register
    }

    fn alloc_register(&mut self) -> u16 {
        let register = self.next_register;
        self.next_register = self
            .next_register
            .checked_add(1)
            .expect("simple expression compiler register index must fit in u16");
        self.max_register = self.max_register.max(self.next_register);
        register
    }

    fn finish(self) -> CompileResult {
        if !self.diagnostics.is_empty() {
            return CompileResult {
                proto: None,
                diagnostics: self.diagnostics,
            };
        }

        let proto = self
            .builder
            .with_signature(self.max_register.max(1), 0, false)
            .finish();
        if let Err(errors) = verify_proto(&proto) {
            return CompileResult {
                proto: None,
                diagnostics: vec![Diagnostic::error(format!(
                    "internal bytecode verification failed: {errors:?}"
                ))],
            };
        }

        CompileResult {
            proto: Some(proto),
            diagnostics: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use elara_bytecode::{Op, disassemble};
    use elara_core::SourceId;
    use elara_test::assert_snapshot_eq;

    use crate::compile_simple_chunk;

    #[test]
    fn simple_expr_compiles_return_arithmetic() {
        let compiled = compile_simple_chunk(SourceId::new(0), "return 1 + 2 * 3");
        assert_eq!(compiled.diagnostics, Vec::new());
        let proto = compiled.proto.expect("expected compiled proto");

        assert_eq!(proto.constants.len(), 3);
        assert_eq!(proto.code.last().map(|instr| instr.op()), Some(Op::Return));
        assert_snapshot_eq(
            disassemble(&proto),
            "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 LOAD_K        A=1 Bx=1 ; 2\n0002 LOAD_K        A=2 Bx=2 ; 3\n0003 MUL           A=1 B=1 C=2\n0004 ADD           A=0 B=0 C=1\n0005 RETURN        A=0 B=1 C=0\n",
        );
    }

    #[test]
    fn simple_expr_compiles_unary_arithmetic() {
        let compiled = compile_simple_chunk(SourceId::new(0), "return -1");
        assert_eq!(compiled.diagnostics, Vec::new());
        let proto = compiled.proto.expect("expected compiled proto");

        assert_snapshot_eq(
            disassemble(&proto),
            "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 UNM           A=0 B=0 C=0\n0002 RETURN        A=0 B=1 C=0\n",
        );
    }

    #[test]
    fn simple_expr_reports_unsupported_statement() {
        let compiled = compile_simple_chunk(SourceId::new(0), "x = 1");

        assert!(compiled.proto.is_none());
        assert_eq!(
            compiled.diagnostics[0].message(),
            "unsupported statement in simple expression compiler"
        );
    }
}
