//! Global declaration and environment lowering for the simple compiler.

use elara_bytecode::Op;
use elara_core::{Diagnostic, SHORT_STRING_MAX_BYTES, Span};
use elara_syntax::{Expr, GlobalDecl, NameDecl};

use super::{GlobalAccess, GlobalDefault, SimpleCompiler};

impl SimpleCompiler {
    pub(super) fn compile_global(&mut self, span: Span, decl: &GlobalDecl<'_>) {
        match decl {
            GlobalDecl::Names { names, values } => self.compile_global_names(span, names, values),
            GlobalDecl::All { attribute } => {
                let access = self.global_access_from_attribute(span, *attribute);
                self.global_default = GlobalDefault::Explicit(access);
            }
        }
    }

    pub(super) fn name_register(&mut self, expr: &Expr<'_>, name: &str) -> u16 {
        if let Some(register) = self.locals.get(name).copied() {
            return register;
        }
        if let Some(upvalue) = self.upvalue_index(name) {
            let register = self.alloc_register();
            self.builder
                .emit_abc(Op::GetUpvalue, register, u32::from(upvalue), 0);
            return register;
        }
        if self.global_access(name).is_some() {
            return self.emit_get_global(name);
        }

        self.diagnostics.push(
            Diagnostic::error(format!("variable '{name}' not declared"))
                .with_primary_span(expr.span()),
        );
        self.alloc_register()
    }

    pub(super) fn compile_name_assignment(&mut self, target: &Expr<'_>, name: &str, value: u16) {
        if let Some(register) = self.locals.get(name).copied() {
            self.emit_move(register, value);
            return;
        }

        match self.global_access(name) {
            Some(GlobalAccess::ReadWrite) => self.emit_set_global(name, value),
            Some(GlobalAccess::ReadOnly) => self.diagnostics.push(
                Diagnostic::error(format!("global variable '{name}' is read-only"))
                    .with_primary_span(target.span()),
            ),
            None => self.diagnostics.push(
                Diagnostic::error(format!("variable '{name}' not declared"))
                    .with_primary_span(target.span()),
            ),
        }
    }

    pub(super) fn declare_global_name(&mut self, name: &str, access: GlobalAccess) {
        if self.global_default == GlobalDefault::PreambularReadWrite {
            self.global_default = GlobalDefault::None;
        }
        self.globals.insert(name.to_owned(), access);
    }

    pub(super) fn emit_set_global(&mut self, name: &str, value: u16) {
        if let Some(name_index) = self.global_name_index(name) {
            self.builder
                .emit_abx(Op::SetEnv, value, u64::from(name_index));
        }
    }

    pub(super) fn emit_global_declaration_check(&mut self, name: &str) {
        let check = self.alloc_register();
        if let Some(name_index) = self.global_name_index(name) {
            self.builder
                .emit_abx(Op::DeclGlobal, check, u64::from(name_index));
        }
    }

    fn compile_global_names(&mut self, span: Span, names: &[NameDecl<'_>], values: &[Expr<'_>]) {
        let value_registers = self.compile_expression_list(values);
        let declarations: Vec<_> = names
            .iter()
            .map(|name| {
                (
                    name.name,
                    self.global_access_from_attribute(name.span, name.attribute),
                )
            })
            .collect();

        for (index, (name, _)) in declarations.iter().copied().enumerate() {
            if values.is_empty() {
                continue;
            }
            let value = value_registers
                .get(index)
                .copied()
                .unwrap_or_else(|| self.emit_nil());
            self.emit_global_declaration_check(name);
            self.emit_set_global(name, value);
        }

        for (name, access) in declarations {
            self.declare_global_name(name, access);
        }

        if names.is_empty() {
            self.diagnostics
                .push(Diagnostic::error("global declaration has no names").with_primary_span(span));
        }
    }

    fn emit_get_global(&mut self, name: &str) -> u16 {
        let register = self.alloc_register();
        if let Some(name_index) = self.global_name_index(name) {
            self.builder
                .emit_abx(Op::GetEnv, register, u64::from(name_index));
        }
        register
    }

    fn global_access(&self, name: &str) -> Option<GlobalAccess> {
        self.globals
            .get(name)
            .copied()
            .or(match self.global_default {
                GlobalDefault::PreambularReadWrite => Some(GlobalAccess::ReadWrite),
                GlobalDefault::Explicit(access) => Some(access),
                GlobalDefault::None => None,
            })
    }

    fn global_access_from_attribute(
        &mut self,
        span: Span,
        attribute: Option<&str>,
    ) -> GlobalAccess {
        match attribute {
            None => GlobalAccess::ReadWrite,
            Some("const") => GlobalAccess::ReadOnly,
            Some("close") => {
                self.diagnostics.push(
                    Diagnostic::error("global variables cannot be to-be-closed")
                        .with_primary_span(span),
                );
                GlobalAccess::ReadWrite
            }
            Some(attribute) => {
                self.diagnostics.push(
                    Diagnostic::error(format!("unsupported global attribute '{attribute}'"))
                        .with_primary_span(span),
                );
                GlobalAccess::ReadWrite
            }
        }
    }

    fn global_name_index(&mut self, name: &str) -> Option<u32> {
        if name.len() > SHORT_STRING_MAX_BYTES {
            self.diagnostics.push(Diagnostic::error(format!(
                "global name '{name}' exceeds current short-string support"
            )));
            return None;
        }
        Some(self.builder.add_string_constant(name.as_bytes()))
    }
}
