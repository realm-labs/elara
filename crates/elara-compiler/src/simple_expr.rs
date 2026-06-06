//! Simple expression bytecode lowering.

use std::collections::HashMap;

use elara_bytecode::{Op, Proto, ProtoBuilder, UpvalueDesc, verify_proto};
use elara_core::{Diagnostic, SourceId, Value};
use elara_syntax::{
    BinaryOp, Expr, ExprKind, FunctionBody, FunctionScope, NameDecl, StmtKind, UnaryOp, parse_chunk,
};

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

    let mut compiler = SimpleCompiler::new();
    compiler.compile_block(parsed.block.statements());
    compiler.finish()
}

struct SimpleCompiler {
    builder: ProtoBuilder,
    diagnostics: Vec<Diagnostic>,
    next_register: u16,
    max_register: u16,
    locals: HashMap<String, u16>,
    enclosing_locals: HashMap<String, u16>,
    upvalues: HashMap<String, u16>,
}

impl SimpleCompiler {
    fn new() -> Self {
        Self {
            builder: ProtoBuilder::new(),
            diagnostics: Vec::new(),
            next_register: 0,
            max_register: 0,
            locals: HashMap::new(),
            enclosing_locals: HashMap::new(),
            upvalues: HashMap::new(),
        }
    }

    fn new_child(enclosing_locals: HashMap<String, u16>) -> Self {
        Self {
            enclosing_locals,
            ..Self::new()
        }
    }

    fn compile_block(&mut self, statements: &[elara_syntax::Stmt<'_>]) {
        for statement in statements {
            match statement.kind() {
                StmtKind::Local { names, values } => {
                    self.compile_local(statement.span(), names, values)
                }
                StmtKind::Assign { targets, values } => {
                    self.compile_assignment(statement.span(), targets, values);
                }
                StmtKind::Function { scope, name, body } => {
                    self.compile_function(statement.span(), *scope, name.base, body);
                }
                StmtKind::Return(values) => self.compile_return(values),
                _ => self.diagnostics.push(
                    Diagnostic::error("unsupported statement in simple expression compiler")
                        .with_primary_span(statement.span()),
                ),
            }
        }
    }

    fn compile_function(
        &mut self,
        span: elara_core::Span,
        _scope: FunctionScope,
        name: &str,
        body: &FunctionBody<'_>,
    ) {
        if !body.params.is_empty() {
            self.diagnostics.push(
                Diagnostic::error("function parameters are not supported yet")
                    .with_primary_span(span),
            );
            return;
        }

        let mut child = SimpleCompiler::new_child(self.locals.clone());
        child.compile_block(body.block.statements());
        let result = child.finish();
        if !result.diagnostics.is_empty() {
            self.diagnostics.extend(result.diagnostics);
            return;
        }

        let register = self.ensure_local(name);
        let child_index = self.builder.add_child(result.proto.expect("child proto"));
        self.builder
            .emit_abx(Op::Closure, register, u64::from(child_index));
    }

    fn compile_return(&mut self, values: &[Expr<'_>]) {
        let registers = self.compile_expression_list(values);
        let start = self.contiguous_return_start(&registers);
        self.builder
            .emit_abc(Op::Return, start, registers.len() as u32, 0);
    }

    fn compile_local(
        &mut self,
        span: elara_core::Span,
        names: &[NameDecl<'_>],
        values: &[Expr<'_>],
    ) {
        let value_registers = self.compile_expression_list(values);
        for (index, name) in names.iter().enumerate() {
            let register = self.alloc_register();
            self.locals.insert(name.name.to_owned(), register);
            if let Some(value) = value_registers.get(index).copied() {
                self.emit_move(register, value);
            } else {
                self.builder.emit_abc(Op::LoadNil, register, 0, 0);
            }
        }

        if names.is_empty() {
            self.diagnostics
                .push(Diagnostic::error("local declaration has no names").with_primary_span(span));
        }
    }

    fn compile_assignment(
        &mut self,
        span: elara_core::Span,
        targets: &[Expr<'_>],
        values: &[Expr<'_>],
    ) {
        let value_registers = self.compile_expression_list(values);
        for (index, target) in targets.iter().enumerate() {
            let Some(register) = self.target_register(target) else {
                continue;
            };

            if let Some(value) = value_registers.get(index).copied() {
                self.emit_move(register, value);
            } else {
                self.builder.emit_abc(Op::LoadNil, register, 0, 0);
            }
        }

        if targets.is_empty() {
            self.diagnostics
                .push(Diagnostic::error("assignment has no targets").with_primary_span(span));
        }
    }

    fn compile_expression_list(&mut self, values: &[Expr<'_>]) -> Vec<u16> {
        values
            .iter()
            .map(|value| self.compile_expr(value))
            .collect()
    }

    fn contiguous_return_start(&mut self, registers: &[u16]) -> u16 {
        let Some((&first, rest)) = registers.split_first() else {
            return 0;
        };

        if rest
            .iter()
            .enumerate()
            .all(|(index, register)| *register == first + index as u16 + 1)
        {
            return first;
        }

        let start = self.next_register;
        for register in registers {
            let target = self.alloc_register();
            self.emit_move(target, *register);
        }
        start
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
            ExprKind::Name(name) => self.local_register(expr, name),
            ExprKind::Call { callee, args, .. } => self.compile_call(expr, callee, args),
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

    fn local_register(&mut self, expr: &Expr<'_>, name: &str) -> u16 {
        if let Some(register) = self.locals.get(name).copied() {
            return register;
        }
        if let Some(upvalue) = self.upvalue_index(name) {
            let register = self.alloc_register();
            self.builder
                .emit_abc(Op::GetUpvalue, register, u32::from(upvalue), 0);
            return register;
        }

        self.diagnostics
            .push(Diagnostic::error("unknown local variable").with_primary_span(expr.span()));
        self.alloc_register()
    }

    fn upvalue_index(&mut self, name: &str) -> Option<u16> {
        if let Some(index) = self.upvalues.get(name).copied() {
            return Some(index);
        }

        let register = self.enclosing_locals.get(name).copied()?;
        let index = self
            .builder
            .add_upvalue(UpvalueDesc::new(Some(name), true, register));
        self.upvalues.insert(name.to_owned(), index);
        Some(index)
    }

    fn ensure_local(&mut self, name: &str) -> u16 {
        if let Some(register) = self.locals.get(name).copied() {
            return register;
        }

        let register = self.alloc_register();
        self.locals.insert(name.to_owned(), register);
        register
    }

    fn compile_call(&mut self, expr: &Expr<'_>, callee: &Expr<'_>, args: &[Expr<'_>]) -> u16 {
        if !args.is_empty() {
            self.diagnostics.push(
                Diagnostic::error("function arguments are not supported yet")
                    .with_primary_span(expr.span()),
            );
            return self.alloc_register();
        }

        let register = self.compile_expr(callee);
        self.builder.emit_abc(Op::Call, register, 1, 1);
        register
    }

    fn target_register(&mut self, target: &Expr<'_>) -> Option<u16> {
        if let ExprKind::Name(name) = target.kind() {
            if let Some(register) = self.locals.get(*name).copied() {
                return Some(register);
            }
            self.diagnostics.push(
                Diagnostic::error("assignment target is not a declared local")
                    .with_primary_span(target.span()),
            );
            return None;
        }

        self.diagnostics.push(
            Diagnostic::error("unsupported assignment target").with_primary_span(target.span()),
        );
        None
    }

    fn emit_move(&mut self, target: u16, source: u16) {
        if target != source {
            self.builder
                .emit_abc(Op::Move, target, u32::from(source), 0);
        }
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

    #[test]
    fn locals_compile_local_return() {
        let compiled = compile_simple_chunk(SourceId::new(0), "local x = 1 + 2\nreturn x");
        assert_eq!(compiled.diagnostics, Vec::new());
        let proto = compiled.proto.expect("expected compiled proto");

        assert_eq!(proto.constants.len(), 2);
        assert_eq!(proto.code.last().map(|instr| instr.op()), Some(Op::Return));
        assert!(disassemble(&proto).contains("MOVE"));
    }

    #[test]
    fn locals_compile_assignment() {
        let compiled = compile_simple_chunk(SourceId::new(0), "local x = 1\nx = x + 2\nreturn x");
        assert_eq!(compiled.diagnostics, Vec::new());
        let proto = compiled.proto.expect("expected compiled proto");

        assert_snapshot_eq(
            disassemble(&proto),
            "0000 LOAD_K        A=0 Bx=0 ; 1\n0001 MOVE          A=1 B=0 C=0\n0002 LOAD_K        A=2 Bx=1 ; 2\n0003 ADD           A=1 B=1 C=2\n0004 RETURN        A=1 B=1 C=0\n",
        );
    }

    #[test]
    fn locals_report_unknown_assignment_target() {
        let compiled = compile_simple_chunk(SourceId::new(0), "x = 1\nreturn x");

        assert!(compiled.proto.is_none());
        assert_eq!(
            compiled.diagnostics[0].message(),
            "assignment target is not a declared local"
        );
    }

    #[test]
    fn functions_compile_local_function_call() {
        let compiled = compile_simple_chunk(
            SourceId::new(0),
            "local function answer()\n  return 42\nend\nreturn answer()",
        );
        assert_eq!(compiled.diagnostics, Vec::new());
        let proto = compiled.proto.expect("expected compiled proto");

        assert_eq!(proto.children.len(), 1);
        assert_snapshot_eq(
            disassemble(&proto),
            "0000 CLOSURE       A=0 Bx=0\n0001 CALL          A=0 B=1 C=1\n0002 RETURN        A=0 B=1 C=0\n",
        );
    }

    #[test]
    fn functions_reject_parameters_for_now() {
        let compiled = compile_simple_chunk(SourceId::new(0), "local function id(x) return x end");

        assert!(compiled.proto.is_none());
        assert_eq!(
            compiled.diagnostics[0].message(),
            "function parameters are not supported yet"
        );
    }

    #[test]
    fn closures_compile_outer_local_capture() {
        let compiled = compile_simple_chunk(
            SourceId::new(0),
            "local x = 41\nlocal function answer()\n  return x + 1\nend\nreturn answer()",
        );
        assert_eq!(compiled.diagnostics, Vec::new());
        let proto = compiled.proto.expect("expected compiled proto");

        assert_eq!(proto.children.len(), 1);
        assert_eq!(proto.children[0].upvalues.len(), 1);
        assert_snapshot_eq(
            disassemble(&proto.children[0]),
            "0000 GET_UPVALUE   A=0 B=0 C=0\n0001 LOAD_K        A=1 Bx=0 ; 1\n0002 ADD           A=0 B=0 C=1\n0003 RETURN        A=0 B=1 C=0\n",
        );
    }
}
