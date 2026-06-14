//! Cranelift lowering for simple integer arithmetic prototypes.

use std::{error::Error, fmt, mem};

use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use elara_bytecode::{Instr, Op, Proto};
use elara_core::{LuaInteger, Value};

use crate::{JitFn, JitRuntimeContext, JitStatus};

/// Compiles and executes a supported arithmetic prototype through Cranelift.
pub fn execute_arithmetic_proto(proto: &Proto) -> Result<Vec<Value>, ArithmeticJitError> {
    let function = ArithmeticJitFunction::compile(proto)?;
    function.execute()
}

/// Compiled simple arithmetic function.
pub struct ArithmeticJitFunction {
    module: JITModule,
    function_id: FuncId,
}

impl ArithmeticJitFunction {
    /// Compiles a simple integer arithmetic prototype.
    pub fn compile(proto: &Proto) -> Result<Self, ArithmeticJitError> {
        let jit_builder = JITBuilder::new(default_libcall_names())?;
        let mut module = JITModule::new(jit_builder);
        let pointer_type = module.target_config().pointer_type();
        let mut context = module.make_context();

        context
            .func
            .signature
            .params
            .push(AbiParam::new(pointer_type));
        context
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I32));

        let mut function_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let context_pointer = builder.block_params(entry)[0];
        let mut known_registers = vec![None; usize::from(proto.max_stack)];
        let variables = (0..proto.max_stack)
            .map(|_| builder.declare_var(types::I64))
            .collect::<Vec<_>>();

        let mut returned = false;
        for (offset, instr) in proto.code.iter().copied().enumerate() {
            if returned {
                return Err(ArithmeticJitError::TrailingInstruction { offset });
            }
            match instr.op() {
                Op::LoadK => {
                    let value = integer_constant(proto, instr, offset)?;
                    let cranelift_value = builder.ins().iconst(types::I64, value);
                    define_register(
                        &mut builder,
                        &variables,
                        &mut known_registers,
                        instr.a(),
                        value,
                        cranelift_value,
                        offset,
                    )?;
                }
                Op::LoadInt => {
                    let value = LuaInteger::from(instr.b());
                    let cranelift_value = builder.ins().iconst(types::I64, value);
                    define_register(
                        &mut builder,
                        &variables,
                        &mut known_registers,
                        instr.a(),
                        value,
                        cranelift_value,
                        offset,
                    )?;
                }
                Op::Move => {
                    let source_register = register_index(instr.b(), offset)?;
                    let value = known_value(&known_registers, source_register, offset)?;
                    let cranelift_value = builder.use_var(variables[source_register]);
                    define_register(
                        &mut builder,
                        &variables,
                        &mut known_registers,
                        instr.a(),
                        value,
                        cranelift_value,
                        offset,
                    )?;
                }
                Op::Add | Op::Sub | Op::Mul => {
                    let left_register = register_index(instr.b(), offset)?;
                    let right_register = register_index(instr.c(), offset)?;
                    let left = known_value(&known_registers, left_register, offset)?;
                    let right = known_value(&known_registers, right_register, offset)?;
                    let value = checked_binary(instr.op(), left, right, offset)?;
                    let left_value = builder.use_var(variables[left_register]);
                    let right_value = builder.use_var(variables[right_register]);
                    let cranelift_value = match instr.op() {
                        Op::Add => builder.ins().iadd(left_value, right_value),
                        Op::Sub => builder.ins().isub(left_value, right_value),
                        Op::Mul => builder.ins().imul(left_value, right_value),
                        _ => unreachable!("opcode checked by outer match"),
                    };
                    define_register(
                        &mut builder,
                        &variables,
                        &mut known_registers,
                        instr.a(),
                        value,
                        cranelift_value,
                        offset,
                    )?;
                }
                Op::AddInt => {
                    let left_register = register_index(instr.b(), offset)?;
                    let right = LuaInteger::from(instr.c());
                    let left = known_value(&known_registers, left_register, offset)?;
                    let value = checked_binary(Op::Add, left, right, offset)?;
                    let left_value = builder.use_var(variables[left_register]);
                    let right_value = builder.ins().iconst(types::I64, right);
                    let cranelift_value = builder.ins().iadd(left_value, right_value);
                    define_register(
                        &mut builder,
                        &variables,
                        &mut known_registers,
                        instr.a(),
                        value,
                        cranelift_value,
                        offset,
                    )?;
                }
                Op::Unm => {
                    let source_register = register_index(instr.b(), offset)?;
                    let value = known_value(&known_registers, source_register, offset)?
                        .checked_neg()
                        .ok_or(ArithmeticJitError::ArithmeticOverflow {
                            offset,
                            op: Op::Unm,
                        })?;
                    let source = builder.use_var(variables[source_register]);
                    let cranelift_value = builder.ins().ineg(source);
                    define_register(
                        &mut builder,
                        &variables,
                        &mut known_registers,
                        instr.a(),
                        value,
                        cranelift_value,
                        offset,
                    )?;
                }
                Op::Return => {
                    if instr.b() != 1 {
                        return Err(ArithmeticJitError::UnsupportedReturnCount {
                            offset,
                            count: instr.b(),
                        });
                    }
                    let return_register = usize::from(instr.a());
                    let value = known_value(&known_registers, return_register, offset)?;
                    let cranelift_value = builder.use_var(variables[return_register]);
                    builder
                        .ins()
                        .store(MemFlags::new(), cranelift_value, context_pointer, 0);
                    let status = builder.ins().iconst(types::I32, JitStatus::Returned as i64);
                    builder.ins().return_(&[status]);
                    known_registers[return_register] = Some(value);
                    returned = true;
                }
                op => {
                    return Err(ArithmeticJitError::UnsupportedOpcode { offset, op });
                }
            }
        }

        if !returned {
            return Err(ArithmeticJitError::MissingReturn);
        }

        builder.finalize();
        let function_id = module.declare_function(
            "elara_arithmetic_proto",
            Linkage::Local,
            &context.func.signature,
        )?;
        module.define_function(function_id, &mut context)?;
        module.clear_context(&mut context);
        module.finalize_definitions()?;

        Ok(Self {
            module,
            function_id,
        })
    }

    /// Executes the compiled function and returns its single integer result.
    pub fn execute(&self) -> Result<Vec<Value>, ArithmeticJitError> {
        let code = self.module.get_finalized_function(self.function_id);
        let function: JitFn = {
            // SAFETY: `code` is the finalized address for a function emitted by
            // `compile` with the exact `JitFn` ABI.
            unsafe { mem::transmute(code) }
        };
        let mut context = ArithmeticRuntimeContext { result: 0 };
        let status = {
            let context_pointer =
                (&mut context as *mut ArithmeticRuntimeContext).cast::<JitRuntimeContext>();
            // SAFETY: The generated function stores only the first `i64` field
            // of `ArithmeticRuntimeContext` and returns a valid `JitStatus`.
            unsafe { function(context_pointer) }
        };
        match status {
            JitStatus::Returned => Ok(vec![Value::integer(context.result)]),
            status => Err(ArithmeticJitError::UnexpectedStatus { status }),
        }
    }
}

#[repr(C)]
struct ArithmeticRuntimeContext {
    result: LuaInteger,
}

fn integer_constant(
    proto: &Proto,
    instr: Instr,
    offset: usize,
) -> Result<LuaInteger, ArithmeticJitError> {
    let index =
        usize::try_from(instr.bx()).map_err(|_| ArithmeticJitError::ConstantOutOfBounds {
            offset,
            index: usize::MAX,
        })?;
    let value = proto
        .constants
        .get(index)
        .copied()
        .ok_or(ArithmeticJitError::ConstantOutOfBounds { offset, index })?;
    value
        .as_integer()
        .ok_or(ArithmeticJitError::UnsupportedConstantValue { offset, index })
}

fn define_register(
    builder: &mut FunctionBuilder<'_>,
    variables: &[Variable],
    known_registers: &mut [Option<LuaInteger>],
    register: u16,
    value: LuaInteger,
    cranelift_value: cranelift_codegen::ir::Value,
    offset: usize,
) -> Result<(), ArithmeticJitError> {
    let register = usize::from(register);
    if register >= known_registers.len() {
        return Err(ArithmeticJitError::RegisterOutOfBounds { offset, register });
    }
    known_registers[register] = Some(value);
    builder.def_var(variables[register], cranelift_value);
    Ok(())
}

fn register_index(register: u32, offset: usize) -> Result<usize, ArithmeticJitError> {
    usize::try_from(register).map_err(|_| ArithmeticJitError::RegisterOutOfBounds {
        offset,
        register: usize::MAX,
    })
}

fn known_value(
    known_registers: &[Option<LuaInteger>],
    register: usize,
    offset: usize,
) -> Result<LuaInteger, ArithmeticJitError> {
    known_registers
        .get(register)
        .copied()
        .flatten()
        .ok_or(ArithmeticJitError::RegisterOutOfBounds { offset, register })
}

fn checked_binary(
    op: Op,
    left: LuaInteger,
    right: LuaInteger,
    offset: usize,
) -> Result<LuaInteger, ArithmeticJitError> {
    match op {
        Op::Add => left.checked_add(right),
        Op::Sub => left.checked_sub(right),
        Op::Mul => left.checked_mul(right),
        _ => None,
    }
    .ok_or(ArithmeticJitError::ArithmeticOverflow { offset, op })
}

/// Error returned when a prototype cannot use the baseline arithmetic JIT.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArithmeticJitError {
    /// Opcode is outside the supported arithmetic subset.
    UnsupportedOpcode {
        /// Instruction offset.
        offset: usize,
        /// Unsupported opcode.
        op: Op,
    },
    /// Constant index is invalid.
    ConstantOutOfBounds {
        /// Instruction offset.
        offset: usize,
        /// Constant index.
        index: usize,
    },
    /// Constant is not an integer.
    UnsupportedConstantValue {
        /// Instruction offset.
        offset: usize,
        /// Constant index.
        index: usize,
    },
    /// Register is invalid or has not been initialized in the supported subset.
    RegisterOutOfBounds {
        /// Instruction offset.
        offset: usize,
        /// Register index.
        register: usize,
    },
    /// Integer arithmetic would overflow and diverge from interpreter behavior.
    ArithmeticOverflow {
        /// Instruction offset.
        offset: usize,
        /// Arithmetic opcode.
        op: Op,
    },
    /// Return count is outside the one-value arithmetic subset.
    UnsupportedReturnCount {
        /// Instruction offset.
        offset: usize,
        /// Requested return count.
        count: u32,
    },
    /// The prototype had no supported return instruction.
    MissingReturn,
    /// Instructions appeared after a return instruction.
    TrailingInstruction {
        /// Instruction offset.
        offset: usize,
    },
    /// Generated code returned an unexpected status.
    UnexpectedStatus {
        /// Returned status.
        status: JitStatus,
    },
    /// Cranelift rejected module construction or finalization.
    Cranelift(String),
}

impl fmt::Display for ArithmeticJitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOpcode { offset, op } => {
                write!(f, "unsupported arithmetic JIT opcode {op:?} at {offset}")
            }
            Self::ConstantOutOfBounds { offset, index } => {
                write!(f, "constant {index} out of bounds at {offset}")
            }
            Self::UnsupportedConstantValue { offset, index } => {
                write!(f, "constant {index} at {offset} is not an integer")
            }
            Self::RegisterOutOfBounds { offset, register } => {
                write!(f, "register {register} unavailable at {offset}")
            }
            Self::ArithmeticOverflow { offset, op } => {
                write!(f, "integer arithmetic overflow for {op:?} at {offset}")
            }
            Self::UnsupportedReturnCount { offset, count } => {
                write!(f, "unsupported return count {count} at {offset}")
            }
            Self::MissingReturn => f.write_str("prototype has no supported return"),
            Self::TrailingInstruction { offset } => {
                write!(f, "instruction after return at {offset}")
            }
            Self::UnexpectedStatus { status } => {
                write!(f, "generated code returned unexpected status {status:?}")
            }
            Self::Cranelift(error) => f.write_str(error),
        }
    }
}

impl Error for ArithmeticJitError {}

impl From<cranelift_module::ModuleError> for ArithmeticJitError {
    fn from(error: cranelift_module::ModuleError) -> Self {
        Self::Cranelift(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use elara_bytecode::{Op, ProtoBuilder};
    use elara_core::Value;
    use elara_interp::execute_proto;

    use super::{ArithmeticJitError, execute_arithmetic_proto};

    #[test]
    fn arithmetic_jit_executes_integer_addition_like_interpreter() {
        let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
        let left = builder.add_constant(Value::integer(20));
        let right = builder.add_constant(Value::integer(22));
        builder.emit_abx(Op::LoadK, 0, u64::from(left));
        builder.emit_abx(Op::LoadK, 1, u64::from(right));
        builder.emit_abc(Op::Add, 2, 0, 1);
        builder.emit_abc(Op::Return, 2, 1, 0);
        let proto = builder.finish();

        assert_eq!(
            execute_arithmetic_proto(&proto),
            execute_proto(&proto).map_err(|error| ArithmeticJitError::Cranelift(error.to_string()))
        );
    }

    #[test]
    fn arithmetic_jit_executes_mixed_integer_ops_like_interpreter() {
        let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
        let ten = builder.add_constant(Value::integer(10));
        builder.emit_abx(Op::LoadK, 0, u64::from(ten));
        builder.emit_abc(Op::LoadInt, 1, 3, 0);
        builder.emit_abc(Op::Sub, 2, 0, 1);
        builder.emit_abc(Op::Move, 3, 2, 0);
        builder.emit_abc(Op::Mul, 3, 3, 1);
        builder.emit_abc(Op::AddInt, 3, 3, 5);
        builder.emit_abc(Op::Unm, 3, 3, 0);
        builder.emit_abc(Op::Return, 3, 1, 0);
        let proto = builder.finish();

        assert_eq!(
            execute_arithmetic_proto(&proto),
            execute_proto(&proto).map_err(|error| ArithmeticJitError::Cranelift(error.to_string()))
        );
    }

    #[test]
    fn arithmetic_jit_rejects_non_integer_constants() {
        let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
        let constant = builder.add_constant(Value::float(1.5));
        builder.emit_abx(Op::LoadK, 0, u64::from(constant));
        builder.emit_abc(Op::Return, 0, 1, 0);

        assert_eq!(
            execute_arithmetic_proto(&builder.finish()),
            Err(ArithmeticJitError::UnsupportedConstantValue {
                offset: 0,
                index: 0,
            })
        );
    }
}
