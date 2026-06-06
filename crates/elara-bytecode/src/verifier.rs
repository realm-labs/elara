//! Bytecode verifier.

use crate::{Instr, Op, Proto, Register};

/// Verifies a prototype.
pub fn verify_proto(proto: &Proto) -> Result<(), Vec<VerifyError>> {
    let mut verifier = Verifier {
        proto,
        errors: Vec::new(),
    };
    verifier.verify();

    if verifier.errors.is_empty() {
        Ok(())
    } else {
        Err(verifier.errors)
    }
}

/// One bytecode verification error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyError {
    /// Instruction offset where the error was found.
    pub offset: usize,
    /// Error kind.
    pub kind: VerifyErrorKind,
}

/// Bytecode verification error payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VerifyErrorKind {
    /// Register operand is outside `Proto::max_stack`.
    RegisterOutOfBounds {
        /// Register operand.
        register: u32,
        /// Prototype stack size.
        max_stack: u16,
    },
    /// Constant index is outside the constant pool.
    ConstantOutOfBounds {
        /// Constant index.
        constant: u64,
        /// Constant pool length.
        constants: usize,
    },
    /// Child prototype index is outside the child prototype list.
    ChildOutOfBounds {
        /// Child prototype index.
        child: u64,
        /// Child prototype count.
        children: usize,
    },
    /// Upvalue index is outside the upvalue descriptor list.
    UpvalueOutOfBounds {
        /// Upvalue index.
        upvalue: u32,
        /// Upvalue descriptor count.
        upvalues: usize,
    },
    /// Jump target does not land on an instruction boundary.
    JumpOutOfBounds {
        /// Computed target offset.
        target: isize,
        /// Instruction count.
        code_len: usize,
    },
    /// Call or return register range exceeds the stack frame.
    CallRangeOutOfBounds {
        /// First register in the range.
        base: Register,
        /// Register count.
        count: u32,
        /// Prototype stack size.
        max_stack: u16,
    },
}

struct Verifier<'a> {
    proto: &'a Proto,
    errors: Vec<VerifyError>,
}

impl Verifier<'_> {
    fn verify(&mut self) {
        for (offset, instr) in self.proto.code.iter().copied().enumerate() {
            self.verify_instruction(offset, instr);
        }
    }

    fn verify_instruction(&mut self, offset: usize, instr: Instr) {
        match instr.op() {
            Op::Move => {
                self.check_register(offset, instr.a());
                self.check_register(offset, instr.b());
            }
            Op::LoadNil
            | Op::LoadBool
            | Op::LoadInt
            | Op::LoadFloat
            | Op::NewTable
            | Op::Len
            | Op::Unm
            | Op::BNot
            | Op::VarargTable
            | Op::Close
            | Op::Tbc => self.check_register(offset, instr.a()),
            Op::Vararg => self.check_vararg(offset, instr),
            Op::Closure => {
                self.check_register(offset, instr.a());
                self.check_child(offset, instr.bx());
            }
            Op::LoadK | Op::DeclGlobal => {
                self.check_register(offset, instr.a());
                self.check_constant(offset, instr.bx());
            }
            Op::GetUpvalue | Op::SetUpvalue => {
                self.check_register(offset, instr.a());
                self.check_upvalue(offset, instr.b());
            }
            Op::GetEnv | Op::SetEnv => {
                self.check_register(offset, instr.a());
            }
            Op::GetIndex | Op::SetIndex => {
                self.check_register(offset, instr.a());
                self.check_register(offset, instr.b());
            }
            Op::GetTable
            | Op::SetTable
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IDiv
            | Op::Mod
            | Op::Pow
            | Op::BAnd
            | Op::BOr
            | Op::BXor
            | Op::Shl
            | Op::Shr
            | Op::TestSet
            | Op::Eq
            | Op::Lt
            | Op::Le => {
                self.check_register(offset, instr.a());
                self.check_register(offset, instr.b());
                self.check_register(offset, instr.c());
            }
            Op::Jmp => self.check_jump(offset, instr),
            Op::ForPrep | Op::ForLoop => {
                self.check_register(offset, instr.a());
                self.check_register_range(offset, instr.a(), 3);
                self.check_jump(offset, instr);
            }
            Op::TForPrep | Op::TForLoop => {
                self.check_register(offset, instr.a());
                self.check_jump(offset, instr);
            }
            Op::TForCall | Op::Test => {
                self.check_register(offset, instr.a());
                self.check_register(offset, instr.b());
            }
            Op::Call | Op::TailCall => self.check_call(offset, instr),
            Op::Return => self.check_return(offset, instr),
        }
    }

    fn check_register(&mut self, offset: usize, register: impl Into<u32>) {
        let register = register.into();
        if register >= u32::from(self.proto.max_stack) {
            self.errors.push(VerifyError {
                offset,
                kind: VerifyErrorKind::RegisterOutOfBounds {
                    register,
                    max_stack: self.proto.max_stack,
                },
            });
        }
    }

    fn check_constant(&mut self, offset: usize, constant: u64) {
        if constant >= self.proto.constants.len() as u64 {
            self.errors.push(VerifyError {
                offset,
                kind: VerifyErrorKind::ConstantOutOfBounds {
                    constant,
                    constants: self.proto.constants.len(),
                },
            });
        }
    }

    fn check_child(&mut self, offset: usize, child: u64) {
        if child >= self.proto.children.len() as u64 {
            self.errors.push(VerifyError {
                offset,
                kind: VerifyErrorKind::ChildOutOfBounds {
                    child,
                    children: self.proto.children.len(),
                },
            });
        }
    }

    fn check_upvalue(&mut self, offset: usize, upvalue: u32) {
        if upvalue >= self.proto.upvalues.len() as u32 {
            self.errors.push(VerifyError {
                offset,
                kind: VerifyErrorKind::UpvalueOutOfBounds {
                    upvalue,
                    upvalues: self.proto.upvalues.len(),
                },
            });
        }
    }

    fn check_jump(&mut self, offset: usize, instr: Instr) {
        let target = offset as isize + 1 + instr.sbx() as isize;
        if target < 0 || target > self.proto.code.len() as isize {
            self.errors.push(VerifyError {
                offset,
                kind: VerifyErrorKind::JumpOutOfBounds {
                    target,
                    code_len: self.proto.code.len(),
                },
            });
        }
    }

    fn check_call(&mut self, offset: usize, instr: Instr) {
        self.check_register(offset, u32::from(instr.a()));
        self.check_register_range(offset, instr.a(), instr.b());
        self.check_register_range(offset, instr.a(), instr.c());
    }

    fn check_return(&mut self, offset: usize, instr: Instr) {
        self.check_register(offset, u32::from(instr.a()));
        self.check_register_range(offset, instr.a(), instr.b());
    }

    fn check_vararg(&mut self, offset: usize, instr: Instr) {
        self.check_register(offset, u32::from(instr.a()));
        self.check_register_range(offset, instr.a(), instr.b());
    }

    fn check_register_range(&mut self, offset: usize, base: Register, count: u32) {
        if count == 0 {
            return;
        }

        let end = u32::from(base) + count - 1;
        if end >= u32::from(self.proto.max_stack) {
            self.errors.push(VerifyError {
                offset,
                kind: VerifyErrorKind::CallRangeOutOfBounds {
                    base,
                    count,
                    max_stack: self.proto.max_stack,
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{Instr, Op, ProtoBuilder, VerifyErrorKind, verify_proto};

    #[test]
    fn verifier_accepts_valid_basic_proto() {
        let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
        let constant = builder.add_constant(Value::integer(1));
        builder.emit_abx(Op::LoadK, 0, u64::from(constant));
        builder.emit_abc(Op::Add, 1, 0, 0);
        builder.emit_abc(Op::Return, 1, 1, 0);
        let proto = builder.finish();

        assert_eq!(verify_proto(&proto), Ok(()));
    }

    #[test]
    fn verifier_rejects_out_of_bounds_registers() {
        let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
        builder.emit_abc(Op::Move, 0, 2, 0);
        let errors = verify_proto(&builder.finish()).unwrap_err();

        assert_eq!(
            errors[0].kind,
            VerifyErrorKind::RegisterOutOfBounds {
                register: 2,
                max_stack: 1,
            }
        );
    }

    #[test]
    fn verifier_rejects_out_of_bounds_constants() {
        let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
        builder.emit_abx(Op::LoadK, 0, 0);
        let errors = verify_proto(&builder.finish()).unwrap_err();

        assert_eq!(
            errors[0].kind,
            VerifyErrorKind::ConstantOutOfBounds {
                constant: 0,
                constants: 0,
            }
        );
    }

    #[test]
    fn verifier_rejects_out_of_bounds_jump_targets() {
        let mut builder = ProtoBuilder::new().with_signature(0, 0, false);
        builder.emit(Instr::asbx(Op::Jmp, 0, 1));
        let errors = verify_proto(&builder.finish()).unwrap_err();

        assert_eq!(
            errors[0].kind,
            VerifyErrorKind::JumpOutOfBounds {
                target: 2,
                code_len: 1,
            }
        );
    }

    #[test]
    fn verifier_rejects_out_of_bounds_call_ranges() {
        let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
        builder.emit_abc(Op::Call, 1, 2, 1);
        let errors = verify_proto(&builder.finish()).unwrap_err();

        assert_eq!(
            errors[0].kind,
            VerifyErrorKind::CallRangeOutOfBounds {
                base: 1,
                count: 2,
                max_stack: 2,
            }
        );
    }

    #[test]
    fn verifier_rejects_out_of_bounds_vararg_ranges() {
        let mut builder = ProtoBuilder::new().with_signature(1, 0, true);
        builder.emit_abc(Op::Vararg, 0, 2, 0);
        let errors = verify_proto(&builder.finish()).unwrap_err();

        assert_eq!(
            errors[0].kind,
            VerifyErrorKind::CallRangeOutOfBounds {
                base: 0,
                count: 2,
                max_stack: 1,
            }
        );
    }

    #[test]
    fn verifier_rejects_out_of_bounds_numeric_for_range() {
        let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
        builder.emit_asbx(Op::ForPrep, 0, 0);
        let errors = verify_proto(&builder.finish()).unwrap_err();

        assert_eq!(
            errors[0].kind,
            VerifyErrorKind::CallRangeOutOfBounds {
                base: 0,
                count: 3,
                max_stack: 2,
            }
        );
    }
}
