//! Simple expression bytecode lowering.

use std::collections::HashMap;

use elara_bytecode::{MAX_B, MAX_C, Op, Proto, ProtoBuilder, UpvalueDesc, verify_proto};
use elara_core::{Diagnostic, SourceId, Span, Value};
use elara_syntax::{
    BinaryOp, Expr, ExprKind, FunctionBody, FunctionScope, NameDecl, Param, StmtKind, UnaryOp,
    parse_chunk,
};

mod control;
mod global;
mod table;

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

    let mut compiler = SimpleCompiler::new_root();
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
    enclosing_upvalues: HashMap<String, u16>,
    upvalues: HashMap<String, u16>,
    globals: HashMap<String, GlobalAccess>,
    global_default: GlobalDefault,
    is_vararg: bool,
    param_count: u8,
    loop_breaks: Vec<Vec<usize>>,
    to_be_closed: Vec<u16>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GlobalAccess {
    ReadWrite,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GlobalDefault {
    PreambularReadWrite,
    None,
    Explicit(GlobalAccess),
}

impl SimpleCompiler {
    fn new_root() -> Self {
        let mut compiler = Self::new();
        let env = compiler
            .builder
            .add_upvalue(UpvalueDesc::new(Some("_ENV"), false, 0));
        compiler.upvalues.insert("_ENV".to_owned(), env);
        compiler
    }

    fn new() -> Self {
        Self {
            builder: ProtoBuilder::new(),
            diagnostics: Vec::new(),
            next_register: 0,
            max_register: 0,
            locals: HashMap::new(),
            enclosing_locals: HashMap::new(),
            enclosing_upvalues: HashMap::new(),
            upvalues: HashMap::new(),
            globals: HashMap::new(),
            global_default: GlobalDefault::PreambularReadWrite,
            is_vararg: false,
            param_count: 0,
            loop_breaks: Vec::new(),
            to_be_closed: Vec::new(),
        }
    }

    fn new_child(
        enclosing_locals: HashMap<String, u16>,
        enclosing_upvalues: HashMap<String, u16>,
        globals: HashMap<String, GlobalAccess>,
        global_default: GlobalDefault,
    ) -> Self {
        Self {
            enclosing_locals,
            enclosing_upvalues,
            globals,
            global_default,
            ..Self::new()
        }
    }

    fn compile_block(&mut self, statements: &[elara_syntax::Stmt<'_>]) {
        for statement in statements {
            match statement.kind() {
                StmtKind::Local { names, values } => {
                    self.compile_local(statement.span(), names, values)
                }
                StmtKind::Global(decl) => self.compile_global(statement.span(), decl),
                StmtKind::Assign { targets, values } => {
                    self.compile_assignment(statement.span(), targets, values);
                }
                StmtKind::Function { scope, name, body } => {
                    self.compile_function(statement.span(), *scope, name.base, body);
                }
                StmtKind::If {
                    clauses,
                    else_block,
                } => self.compile_if(clauses, else_block.as_ref()),
                StmtKind::While { condition, body } => self.compile_while(condition, body),
                StmtKind::Repeat { body, condition } => self.compile_repeat(body, condition),
                StmtKind::NumericFor {
                    name,
                    init,
                    limit,
                    step,
                    body,
                } => self.compile_numeric_for(name, init, limit, step.as_ref(), body),
                StmtKind::GenericFor {
                    names,
                    values,
                    body,
                } => self.compile_generic_for(names, values, body),
                StmtKind::Break => self.compile_break(statement.span()),
                StmtKind::Call(expr) => self.compile_call_statement(expr),
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
        span: Span,
        scope: FunctionScope,
        name: &str,
        body: &FunctionBody<'_>,
    ) {
        let mut fixed_params = Vec::new();
        let mut named_vararg = None;
        for param in &body.params {
            match param {
                Param::Name(name) if named_vararg.is_none() => fixed_params.push(*name),
                Param::Vararg(name) if named_vararg.is_none() => named_vararg = Some(*name),
                _ => {
                    self.diagnostics.push(
                        Diagnostic::error("unsupported function parameter list")
                            .with_primary_span(span),
                    );
                    return;
                }
            }
        }

        let Ok(param_count) = u8::try_from(fixed_params.len()) else {
            self.diagnostics.push(
                Diagnostic::error("too many function parameters").with_primary_span(span),
            );
            return;
        };

        if scope == FunctionScope::Global {
            self.declare_global_name(name, GlobalAccess::ReadWrite);
        }

        let register = match scope {
            FunctionScope::Local | FunctionScope::Plain => self.ensure_local(name),
            FunctionScope::Global => self.alloc_register(),
        };
        let mut child = SimpleCompiler::new_child(
            self.locals.clone(),
            self.upvalues.clone(),
            self.globals.clone(),
            self.global_default,
        );
        child.param_count = param_count;
        for param in fixed_params {
            child.ensure_local(param);
        }
        child.is_vararg = named_vararg.is_some();
        if let Some(Some(name)) = named_vararg {
            child.define_named_vararg_table(name);
        }
        child.compile_block(body.block.statements());
        let result = child.finish();
        if !result.diagnostics.is_empty() {
            self.diagnostics.extend(result.diagnostics);
            return;
        }

        let child_index = self.builder.add_child(result.proto.expect("child proto"));
        self.builder
            .emit_abx(Op::Closure, register, u64::from(child_index));
        if scope == FunctionScope::Global {
            self.emit_global_declaration_check(name);
            self.emit_set_global(name, register);
        }
    }

    fn define_named_vararg_table(&mut self, name: &str) {
        let register = self.ensure_local(name);
        self.builder.emit_abc(Op::VarargTable, register, 0, 0);
    }

    fn compile_return(&mut self, values: &[Expr<'_>]) {
        if let Some((last, prefix)) = values.split_last()
            && let ExprKind::Call { callee, args, .. } = last.kind()
        {
            let registers = self.compile_expression_list(prefix);
            let start = if registers.is_empty() {
                self.next_register
            } else {
                self.contiguous_return_start(&registers)
            };
            let call_register = start
                .checked_add(
                    u16::try_from(registers.len())
                        .expect("return expression count must fit in register range"),
                )
                .expect("return call register must fit in register range");
            self.ensure_register_slot(call_register);
            self.compile_call_into_register(last, callee, args, call_register, 0);
            self.emit_close_all();
            self.builder.emit_abc(Op::Return, start, 0, 0);
            return;
        }

        let registers = self.compile_expression_list(values);
        let start = self.contiguous_return_start(&registers);
        self.emit_close_all();
        self.builder
            .emit_abc(Op::Return, start, registers.len() as u32, 0);
    }

    fn compile_local(&mut self, span: Span, names: &[NameDecl<'_>], values: &[Expr<'_>]) {
        let value_registers = self.compile_fixed_expression_list(values, names.len());
        for (index, name) in names.iter().enumerate() {
            let register = self.alloc_register();
            self.locals.insert(name.name.to_owned(), register);
            if let Some(value) = value_registers.get(index).copied() {
                self.emit_move(register, value);
            } else {
                self.builder.emit_abc(Op::LoadNil, register, 0, 0);
            }
            match name.attribute {
                Some("close") => {
                    self.builder.emit_abc(Op::Tbc, register, 0, 0);
                    self.to_be_closed.push(register);
                }
                Some("const") | None => {}
                Some(attribute) => self.diagnostics.push(
                    Diagnostic::error(format!("unsupported local attribute '{attribute}'"))
                        .with_primary_span(name.span),
                ),
            }
            self.record_local_var(name.name, register);
        }

        if names.is_empty() {
            self.diagnostics
                .push(Diagnostic::error("local declaration has no names").with_primary_span(span));
        }
    }

    fn compile_assignment(&mut self, span: Span, targets: &[Expr<'_>], values: &[Expr<'_>]) {
        let value_registers = self.compile_fixed_expression_list(values, targets.len());
        for (index, target) in targets.iter().enumerate() {
            let value = value_registers
                .get(index)
                .copied()
                .unwrap_or_else(|| self.emit_nil());
            self.compile_assignment_target(target, value);
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

    fn compile_fixed_expression_list(
        &mut self,
        values: &[Expr<'_>],
        target_count: usize,
    ) -> Vec<u16> {
        let Some((last, prefix)) = values.split_last() else {
            return Vec::new();
        };

        let mut registers = Vec::new();
        for value in prefix {
            let register = self.compile_expr(value);
            if registers.len() < target_count {
                registers.push(register);
            }
        }

        let remaining = target_count.saturating_sub(registers.len());
        if remaining == 0 {
            self.compile_expr(last);
            return registers;
        }

        match last.kind() {
            ExprKind::Call { callee, args, .. } => {
                let register = self.next_register;
                self.ensure_fixed_result_slots(register, remaining);
                self.compile_call_into_register(last, callee, args, register, remaining as u32);
                registers.extend((0..remaining).map(|offset| register + offset as u16));
            }
            ExprKind::Vararg => {
                let register = self.next_register;
                self.ensure_fixed_result_slots(register, remaining);
                self.compile_vararg_into_register(last, register, remaining as u32);
                registers.extend((0..remaining).map(|offset| register + offset as u16));
            }
            _ => registers.push(self.compile_expr(last)),
        }

        registers
    }

    fn ensure_fixed_result_slots(&mut self, register: u16, count: usize) {
        if count == 0 {
            return;
        }
        let last = register
            .checked_add(u16::try_from(count - 1).expect("result count must fit in registers"))
            .expect("result register range must fit in registers");
        self.ensure_register_slot(last);
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
            ExprKind::String(text) => self.compile_string_literal(expr, text),
            ExprKind::StringKey(text) => self.compile_string_key(expr.span(), text),
            ExprKind::Name(name) => self.name_register(expr, name),
            ExprKind::Vararg => self.compile_vararg(expr),
            ExprKind::Call { callee, args, .. } => self.compile_call(expr, callee, args),
            ExprKind::Index { table, key } => self.compile_table_get(table, key),
            ExprKind::Table(fields) => self.compile_table_constructor(fields),
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

    fn compile_vararg(&mut self, expr: &Expr<'_>) -> u16 {
        let register = self.alloc_register();
        self.compile_vararg_into_register(expr, register, 1);
        register
    }

    fn compile_vararg_into_register(&mut self, expr: &Expr<'_>, register: u16, result_count: u32) {
        if self.is_vararg {
            self.builder.emit_abc(Op::Vararg, register, result_count, 0);
        } else {
            self.diagnostics.push(
                Diagnostic::error("vararg expression outside vararg function")
                    .with_primary_span(expr.span()),
            );
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

    fn emit_nil(&mut self) -> u16 {
        let register = self.alloc_register();
        self.builder.emit_abc(Op::LoadNil, register, 0, 0);
        register
    }

    fn upvalue_index(&mut self, name: &str) -> Option<u16> {
        if let Some(index) = self.upvalues.get(name).copied() {
            return Some(index);
        }

        let (in_stack, parent_index) = if let Some(register) = self.enclosing_locals.get(name) {
            (true, *register)
        } else {
            let upvalue = self.enclosing_upvalues.get(name).copied()?;
            (false, upvalue)
        };
        let index = self
            .builder
            .add_upvalue(UpvalueDesc::new(Some(name), in_stack, parent_index));
        self.upvalues.insert(name.to_owned(), index);
        Some(index)
    }

    fn ensure_local(&mut self, name: &str) -> u16 {
        if let Some(register) = self.locals.get(name).copied() {
            return register;
        }

        let register = self.alloc_register();
        self.locals.insert(name.to_owned(), register);
        self.record_local_var(name, register);
        register
    }

    fn record_local_var(&mut self, name: &str, register: u16) {
        let start_pc = u32::try_from(self.builder.code_len()).expect("pc must fit in u32");
        self.builder
            .add_local_var(name, register, start_pc, u32::MAX);
    }

    fn compile_call(&mut self, expr: &Expr<'_>, callee: &Expr<'_>, args: &[Expr<'_>]) -> u16 {
        self.compile_call_with_result_count(expr, callee, args, 1)
    }

    fn compile_call_statement(&mut self, expr: &Expr<'_>) {
        if let ExprKind::Call { callee, args, .. } = expr.kind() {
            self.compile_call_with_result_count(expr, callee, args, 1);
        } else {
            self.diagnostics.push(
                Diagnostic::error("expected function call statement")
                    .with_primary_span(expr.span()),
            );
        }
    }

    fn compile_call_with_result_count(
        &mut self,
        expr: &Expr<'_>,
        callee: &Expr<'_>,
        args: &[Expr<'_>],
        result_count: u32,
    ) -> u16 {
        let callee = self.compile_expr(callee);
        let register = self.alloc_register();
        self.emit_move(register, callee);
        self.compile_call_args_and_emit(expr, register, args, result_count);
        register
    }

    pub(super) fn compile_call_into_register(
        &mut self,
        expr: &Expr<'_>,
        callee: &Expr<'_>,
        args: &[Expr<'_>],
        register: u16,
        result_count: u32,
    ) {
        let callee = self.compile_expr(callee);
        self.emit_move(register, callee);
        self.compile_call_args_and_emit(expr, register, args, result_count);
    }

    fn compile_call_args_and_emit(
        &mut self,
        expr: &Expr<'_>,
        register: u16,
        args: &[Expr<'_>],
        result_count: u32,
    ) {
        let arg_count = match u32::try_from(args.len() + 1) {
            Ok(count) => count,
            Err(_) => {
                self.diagnostics.push(
                    Diagnostic::error("too many function arguments").with_primary_span(expr.span()),
                );
                return;
            }
        };

        for (index, arg) in args.iter().enumerate() {
            let value = self.compile_expr(arg);
            let Some(target) = u16::try_from(index + 1)
                .ok()
                .and_then(|offset| register.checked_add(offset))
            else {
                self.diagnostics.push(
                    Diagnostic::error("too many function arguments").with_primary_span(expr.span()),
                );
                return;
            };
            self.ensure_register_slot(target);
            self.emit_move(target, value);
        }

        if arg_count > MAX_B {
            self.diagnostics.push(
                Diagnostic::error("too many function arguments").with_primary_span(expr.span()),
            );
            return;
        }

        if result_count > 0 {
            let last_result = register
                .checked_add(
                    u16::try_from(result_count - 1)
                        .expect("call result count must fit in register range"),
                )
                .expect("call result range must fit in register range");
            self.ensure_register_slot(last_result);
        }

        self.builder
            .emit_abc(Op::Call, register, arg_count, result_count);
    }

    fn emit_move(&mut self, target: u16, source: u16) {
        if target != source {
            self.builder
                .emit_abc(Op::Move, target, u32::from(source), 0);
        }
    }

    fn emit_close_all(&mut self) {
        if let Some(register) = self.to_be_closed.first().copied() {
            self.builder.emit_abc(Op::Close, register, 0, 0);
            self.to_be_closed.clear();
        }
    }

    fn ensure_register_slot(&mut self, register: u16) {
        let next = register
            .checked_add(1)
            .expect("simple expression compiler register index must fit in u16");
        self.max_register = self.max_register.max(next);
        self.next_register = self.next_register.max(next);
    }

    fn compile_unary(&mut self, expr: &Expr<'_>, op: UnaryOp) -> u16 {
        let value = self.compile_expr(expr);
        let result = self.alloc_register();
        let bytecode_op = match op {
            UnaryOp::Neg => Op::Unm,
            UnaryOp::BitNot => Op::BNot,
            UnaryOp::Len => Op::Len,
            UnaryOp::Not => Op::Not,
        };
        self.builder.emit_abc(bytecode_op, result, value.into(), 0);
        result
    }

    fn compile_binary(
        &mut self,
        expr: &Expr<'_>,
        op: BinaryOp,
        left: &Expr<'_>,
        right: &Expr<'_>,
    ) -> u16 {
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.compile_logical(op, left, right);
        }

        if op == BinaryOp::Add
            && let Some(immediate) = add_int_immediate(right)
        {
            let left_register = self.compile_expr(left);
            self.builder.emit_abc(
                Op::AddInt,
                left_register,
                u32::from(left_register),
                immediate,
            );
            return left_register;
        }

        let left_register = self.compile_expr(left);
        let right_register = self.compile_expr(right);
        if let Some((bytecode_op, left_operand, right_operand)) =
            comparison_operands(op, left_register, right_register)
        {
            self.builder.emit_abc(
                bytecode_op,
                left_register,
                u32::from(left_operand),
                u32::from(right_operand),
            );
            if op == BinaryOp::Ne {
                let false_register = self.alloc_register();
                self.builder.emit_abc(Op::LoadBool, false_register, 0, 0);
                self.builder.emit_abc(
                    Op::Eq,
                    left_register,
                    u32::from(left_register),
                    u32::from(false_register),
                );
            }
            return left_register;
        }

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
            BinaryOp::Concat => Op::Concat,
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

    fn finish(mut self) -> CompileResult {
        if !self.diagnostics.is_empty() {
            return CompileResult {
                proto: None,
                diagnostics: self.diagnostics,
            };
        }
        self.emit_close_all();

        let proto = self
            .builder
            .with_signature(self.max_register.max(1), self.param_count, self.is_vararg)
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

fn add_int_immediate(expr: &Expr<'_>) -> Option<u32> {
    let ExprKind::Integer(text) = expr.kind() else {
        return None;
    };
    let value = text.parse::<u32>().ok()?;
    (value <= MAX_C).then_some(value)
}

fn comparison_operands(op: BinaryOp, left: u16, right: u16) -> Option<(Op, u16, u16)> {
    match op {
        BinaryOp::Eq | BinaryOp::Ne => Some((Op::Eq, left, right)),
        BinaryOp::Lt => Some((Op::Lt, left, right)),
        BinaryOp::Le => Some((Op::Le, left, right)),
        BinaryOp::Gt => Some((Op::Lt, right, left)),
        BinaryOp::Ge => Some((Op::Le, right, left)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
