//! Control-flow lowering for the simple compiler.

use elara_bytecode::Op;
use elara_core::Diagnostic;
use elara_syntax::{Block, Expr, IfClause};

use super::SimpleCompiler;

impl SimpleCompiler {
    pub(super) fn compile_if(&mut self, clauses: &[IfClause<'_>], else_block: Option<&Block<'_>>) {
        let mut end_jumps = Vec::new();

        for (index, clause) in clauses.iter().enumerate() {
            let condition = self.compile_expr(&clause.condition);
            self.builder.emit_abc(Op::Test, condition, 0, 0);
            let false_jump = self.emit_jump_placeholder();
            self.compile_block(clause.block.statements());

            let has_following_clause = index + 1 < clauses.len();
            if has_following_clause || else_block.is_some() {
                end_jumps.push(self.emit_jump_placeholder());
            }
            self.patch_jump_to_here(false_jump);
        }

        if let Some(block) = else_block {
            self.compile_block(block.statements());
        }

        for jump in end_jumps {
            self.patch_jump_to_here(jump);
        }
    }

    pub(super) fn compile_while(&mut self, condition: &Expr<'_>, body: &Block<'_>) {
        let loop_start = self.builder.code_len();
        let condition = self.compile_expr(condition);
        self.builder.emit_abc(Op::Test, condition, 0, 0);
        let exit_jump = self.emit_jump_placeholder();

        self.push_loop();
        self.compile_block(body.statements());
        let breaks = self.pop_loop();

        self.emit_jump_to(loop_start);
        self.patch_jump_to_here(exit_jump);
        self.patch_breaks(breaks);
    }

    pub(super) fn compile_repeat(&mut self, body: &Block<'_>, condition: &Expr<'_>) {
        let loop_start = self.builder.code_len();

        self.push_loop();
        self.compile_block(body.statements());
        let breaks = self.pop_loop();

        let condition = self.compile_expr(condition);
        self.builder.emit_abc(Op::Test, condition, 0, 0);
        self.emit_jump_to(loop_start);
        self.patch_breaks(breaks);
    }

    pub(super) fn compile_break(&mut self, span: elara_core::Span) {
        let jump = self.emit_jump_placeholder();
        if let Some(loop_breaks) = self.loop_breaks.last_mut() {
            loop_breaks.push(jump);
        } else {
            self.diagnostics
                .push(Diagnostic::error("break outside loop").with_primary_span(span));
        }
    }

    fn push_loop(&mut self) {
        self.loop_breaks.push(Vec::new());
    }

    fn pop_loop(&mut self) -> Vec<usize> {
        self.loop_breaks
            .pop()
            .expect("loop break stack must contain current loop")
    }

    fn patch_breaks(&mut self, breaks: Vec<usize>) {
        for jump in breaks {
            self.patch_jump_to_here(jump);
        }
    }

    fn emit_jump_placeholder(&mut self) -> usize {
        self.builder.emit_asbx(Op::Jmp, 0, 0)
    }

    fn emit_jump_to(&mut self, target: usize) {
        let jump = self.emit_jump_placeholder();
        self.patch_jump(jump, target);
    }

    fn patch_jump_to_here(&mut self, offset: usize) {
        self.patch_jump(offset, self.builder.code_len());
    }

    fn patch_jump(&mut self, offset: usize, target: usize) {
        let sbx = i64::try_from(target).expect("jump target must fit in i64")
            - i64::try_from(offset + 1).expect("jump origin must fit in i64");
        self.builder.patch_asbx(offset, Op::Jmp, 0, sbx);
    }
}
