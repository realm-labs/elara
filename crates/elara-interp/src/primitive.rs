//! Primitive bytecode execution.

use elara_bytecode::{Instr, Op, Proto, VerifyError, verify_proto};
use elara_core::{GcArena, LuaFloat, LuaInteger, LuaThread, StringInterner, Value};

mod loops;
mod table;

use loops::{
    execute_generic_for_call, execute_generic_for_loop, execute_numeric_for_loop,
    prepare_numeric_for,
};
pub use table::RuntimeTables;
use table::{
    execute_get_index, execute_get_table, execute_new_table, execute_set_index, execute_set_table,
    execute_vararg_table,
};

/// Result of executing one prototype.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Values and temporary runtime-owned tables produced by primitive execution.
pub struct RuntimeOutput {
    /// Returned Lua values.
    pub values: Vec<Value>,
    /// Runtime table storage referenced by table placeholder values.
    pub tables: RuntimeTables,
    /// Runtime string storage referenced by string values.
    pub strings: RuntimeStrings,
}

/// Runtime-owned string storage for primitive execution output.
#[derive(Default)]
pub struct RuntimeStrings {
    arena: GcArena,
    interner: StringInterner,
}

impl RuntimeStrings {
    /// Creates empty runtime string storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a short string and returns it as a Lua value.
    pub fn intern_short_value(&mut self, bytes: impl AsRef<[u8]>) -> Value {
        Value::short_string(self.interner.intern_short(&mut self.arena, bytes))
    }
}

/// Primitive interpreter runtime error.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeError {
    /// Bytecode verifier rejected the prototype.
    Verification(Vec<VerifyError>),
    /// Instruction tried to read an invalid constant.
    ConstantOutOfBounds { index: usize },
    /// Instruction tried to access an invalid register.
    RegisterOutOfBounds { register: usize },
    /// Instruction tried to access an invalid string constant.
    StringOutOfBounds { index: usize },
    /// Arithmetic operand was not numeric.
    NonNumericOperand { op: Op },
    /// Table operation received a non-table receiver.
    NonTableValue,
    /// Table write used an invalid Lua key.
    InvalidTableKey,
    /// Metamethod dispatch found a metamethod shape this interpreter cannot call yet.
    UnsupportedMetamethod { name: &'static str },
    /// Metamethod table chain exceeded Lua's loop limit.
    MetamethodChainTooLong { name: &'static str },
    /// Call operand was not callable.
    NonCallableValue,
    /// Closure referenced a missing child prototype.
    ChildOutOfBounds { index: usize },
    /// Upvalue read referenced a missing captured value.
    UpvalueOutOfBounds { index: usize },
    /// Jump instruction computed an invalid program counter.
    JumpOutOfBounds { target: isize },
    /// Numeric for-loop operand was not numeric.
    ForLoopNonNumeric { operand: &'static str },
    /// Numeric for-loop step was zero.
    ForLoopStepZero,
    /// Numeric for-loop iteration count exceeded the current runtime storage.
    ForLoopCountOverflow,
    /// Opcode is not supported by the primitive interpreter.
    UnsupportedOpcode { op: Op },
}

/// Executes a verified prototype and returns the first return values.
pub fn execute_proto(proto: &Proto) -> RuntimeResult<Vec<Value>> {
    execute_proto_with_output(proto).map(|output| output.values)
}

/// Executes a verified prototype and returns values plus runtime-owned tables.
pub fn execute_proto_with_output(proto: &Proto) -> RuntimeResult<RuntimeOutput> {
    verify_proto(proto).map_err(RuntimeError::Verification)?;
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let values =
        execute_proto_with_upvalues(proto, &[], &[], &mut closures, &mut tables, &mut strings)?;
    Ok(RuntimeOutput {
        values,
        tables,
        strings,
    })
}

fn execute_proto_with_upvalues(
    proto: &Proto,
    upvalues: &[Value],
    varargs: &[Value],
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Vec<Value>> {
    let mut thread = LuaThread::new();
    for _ in 0..proto.max_stack {
        thread.push_value(Value::nil());
    }
    let mut dynamic_top = 0;

    let mut pc = 0;
    while pc < proto.code.len() {
        let instr = proto.code[pc];
        pc += 1;

        match instr.op() {
            Op::Move => {
                let value = register(&thread, instr.b() as usize)?;
                set_register(&mut thread, instr.a().into(), value)?;
            }
            Op::LoadNil => set_register(&mut thread, instr.a().into(), Value::nil())?,
            Op::LoadBool => {
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::boolean(instr.b() != 0),
                )?;
            }
            Op::LoadInt => {
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::integer(LuaInteger::from(instr.b())),
                )?;
            }
            Op::LoadFloat => {
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::float(LuaFloat::from(instr.b())),
                )?;
            }
            Op::LoadK => {
                let constant = proto.constants.get(instr.bx() as usize).copied().ok_or(
                    RuntimeError::ConstantOutOfBounds {
                        index: instr.bx() as usize,
                    },
                )?;
                set_register(&mut thread, instr.a().into(), constant)?;
            }
            Op::LoadString => {
                let string = proto.string_constants.get(instr.bx() as usize).ok_or(
                    RuntimeError::StringOutOfBounds {
                        index: instr.bx() as usize,
                    },
                )?;
                let value = strings.intern_short_value(string);
                set_register(&mut thread, instr.a().into(), value)?;
            }
            Op::NewTable => execute_new_table(&mut thread, instr, tables)?,
            Op::GetTable => execute_get_table(&mut thread, closures, instr, tables, strings)?,
            Op::SetTable => execute_set_table(&mut thread, closures, instr, tables, strings)?,
            Op::GetIndex => execute_get_index(&mut thread, closures, instr, tables, strings)?,
            Op::SetIndex => execute_set_index(&mut thread, closures, instr, tables, strings)?,
            Op::GetUpvalue => {
                let value = upvalues.get(instr.b() as usize).copied().ok_or(
                    RuntimeError::UpvalueOutOfBounds {
                        index: instr.b() as usize,
                    },
                )?;
                set_register(&mut thread, instr.a().into(), value)?;
            }
            Op::Closure => {
                let child_index = instr.bx() as usize;
                let child = proto
                    .children
                    .get(child_index)
                    .cloned()
                    .ok_or(RuntimeError::ChildOutOfBounds { index: child_index })?;
                let closure_index = closures.len();
                closures.push(RuntimeClosure {
                    proto: child.clone(),
                    upvalues: Vec::new(),
                });
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::closure_index(closure_index as u32),
                )?;
                let captured = capture_upvalues(&child, &thread, upvalues)?;
                closures[closure_index].upvalues = captured;
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::IDiv | Op::Mod | Op::Pow | Op::Unm => {
                execute_arithmetic(&mut thread, instr)?
            }
            Op::Jmp => pc = jump_target(pc, instr)?,
            Op::ForPrep => {
                if prepare_numeric_for(&mut thread, instr)? {
                    pc = jump_target(pc, instr)?;
                }
            }
            Op::ForLoop => {
                if execute_numeric_for_loop(&mut thread, instr)? {
                    pc = jump_target(pc, instr)?;
                }
            }
            Op::TForPrep => pc = jump_target(pc, instr)?,
            Op::TForCall => {
                execute_generic_for_call(&mut thread, closures, instr, tables, strings)?;
            }
            Op::TForLoop => {
                if execute_generic_for_loop(&mut thread, instr)? {
                    pc = jump_target(pc, instr)?;
                }
            }
            Op::Test => {
                let value = register(&thread, instr.a().into())?;
                if is_truthy(value) != (instr.b() != 0) {
                    pc += 1;
                }
            }
            Op::Vararg => {
                if let Some(top) = execute_vararg(&mut thread, instr, varargs)? {
                    dynamic_top = top;
                }
            }
            Op::VarargTable => execute_vararg_table(&mut thread, instr, varargs, tables)?,
            Op::Call => {
                if let Some(top) = execute_call(&mut thread, closures, instr, tables, strings)? {
                    dynamic_top = top;
                }
            }
            Op::Return => return collect_returns(&thread, instr, dynamic_top),
            op => return Err(RuntimeError::UnsupportedOpcode { op }),
        }
    }

    Ok(Vec::new())
}

fn execute_arithmetic(thread: &mut LuaThread, instr: Instr) -> RuntimeResult<()> {
    let op = instr.op();
    if op == Op::Unm {
        let value = register(thread, instr.b() as usize)?;
        let result = negate(value).ok_or(RuntimeError::NonNumericOperand { op })?;
        return set_register(thread, instr.a().into(), result);
    }

    let left = register(thread, instr.b() as usize)?;
    let right = register(thread, instr.c() as usize)?;
    let result =
        binary_arithmetic(op, left, right).ok_or(RuntimeError::NonNumericOperand { op })?;
    set_register(thread, instr.a().into(), result)
}

fn jump_target(pc: usize, instr: Instr) -> RuntimeResult<usize> {
    let target = pc as isize + instr.sbx() as isize;
    usize::try_from(target).map_err(|_| RuntimeError::JumpOutOfBounds { target })
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

#[derive(Clone, Debug)]
struct RuntimeClosure {
    proto: Proto,
    upvalues: Vec<Value>,
}

fn capture_upvalues(
    child: &Proto,
    thread: &LuaThread,
    parent_upvalues: &[Value],
) -> RuntimeResult<Vec<Value>> {
    let mut captured = Vec::with_capacity(child.upvalues.len());
    for upvalue in &child.upvalues {
        let value = if upvalue.in_stack {
            register(thread, usize::from(upvalue.index))?
        } else {
            parent_upvalues
                .get(usize::from(upvalue.index))
                .copied()
                .ok_or(RuntimeError::UpvalueOutOfBounds {
                    index: usize::from(upvalue.index),
                })?
        };
        captured.push(value);
    }
    Ok(captured)
}

fn execute_vararg(
    thread: &mut LuaThread,
    instr: Instr,
    varargs: &[Value],
) -> RuntimeResult<Option<usize>> {
    let base = usize::from(instr.a());
    let count = if instr.b() == 0 {
        varargs.len()
    } else {
        instr.b() as usize
    };

    for index in 0..count {
        let value = varargs.get(index).copied().unwrap_or_else(Value::nil);
        set_register(thread, base + index, value)?;
    }

    Ok((instr.b() == 0).then_some(base + count))
}

fn execute_call(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<usize>> {
    let callee = register(thread, instr.a().into())?;
    let closure_index = callee
        .as_closure_index()
        .ok_or(RuntimeError::NonCallableValue)? as usize;
    let args = collect_call_args(thread, instr)?;
    let returns = call_closure(closures, closure_index, &args, tables, strings)?;

    let base = usize::from(instr.a());
    let count = if instr.c() == 0 {
        returns.len()
    } else {
        instr.c() as usize
    };

    for index in 0..count {
        let value = returns.get(index).copied().unwrap_or_else(Value::nil);
        set_register(thread, base + index, value)?;
    }

    Ok((instr.c() == 0).then_some(base + count))
}

fn call_closure(
    closures: &mut Vec<RuntimeClosure>,
    closure_index: usize,
    args: &[Value],
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Vec<Value>> {
    let closure = closures
        .get(closure_index)
        .cloned()
        .ok_or(RuntimeError::NonCallableValue)?;
    execute_proto_with_upvalues(
        &closure.proto,
        &closure.upvalues,
        args,
        closures,
        tables,
        strings,
    )
}

fn collect_call_args(thread: &LuaThread, instr: Instr) -> RuntimeResult<Vec<Value>> {
    let count = instr.b().saturating_sub(1);
    let mut args = Vec::with_capacity(count as usize);
    let base = usize::from(instr.a()) + 1;
    for index in 0..count {
        args.push(register(thread, base + index as usize)?);
    }
    Ok(args)
}

fn binary_arithmetic(op: Op, left: Value, right: Value) -> Option<Value> {
    match (left.as_integer(), right.as_integer()) {
        (Some(left), Some(right)) => integer_arithmetic(op, left, right),
        _ => float_arithmetic(op, left.to_float()?, right.to_float()?),
    }
}

fn integer_arithmetic(op: Op, left: LuaInteger, right: LuaInteger) -> Option<Value> {
    match op {
        Op::Add => left.checked_add(right).map(Value::integer),
        Op::Sub => left.checked_sub(right).map(Value::integer),
        Op::Mul => left.checked_mul(right).map(Value::integer),
        Op::IDiv => (right != 0).then(|| Value::integer(left.div_euclid(right))),
        Op::Mod => (right != 0).then(|| Value::integer(left.rem_euclid(right))),
        Op::Div | Op::Pow => float_arithmetic(op, left as LuaFloat, right as LuaFloat),
        _ => None,
    }
}

fn float_arithmetic(op: Op, left: LuaFloat, right: LuaFloat) -> Option<Value> {
    let value = match op {
        Op::Add => left + right,
        Op::Sub => left - right,
        Op::Mul => left * right,
        Op::Div => left / right,
        Op::IDiv => (left / right).floor(),
        Op::Mod => left - (left / right).floor() * right,
        Op::Pow => left.powf(right),
        _ => return None,
    };
    Some(Value::float(value))
}

fn negate(value: Value) -> Option<Value> {
    if let Some(value) = value.as_integer() {
        return value.checked_neg().map(Value::integer);
    }
    Some(Value::float(-value.to_float()?))
}

fn collect_returns(
    thread: &LuaThread,
    instr: Instr,
    dynamic_top: usize,
) -> RuntimeResult<Vec<Value>> {
    let base = usize::from(instr.a());
    let count = if instr.b() == 0 {
        dynamic_top.saturating_sub(base)
    } else {
        instr.b() as usize
    };
    let mut values = Vec::with_capacity(count);
    for index in base..base + count {
        values.push(register(thread, index)?);
    }
    Ok(values)
}

fn register(thread: &LuaThread, index: usize) -> RuntimeResult<Value> {
    thread
        .stack_value(index)
        .ok_or(RuntimeError::RegisterOutOfBounds { register: index })
}

fn set_register(thread: &mut LuaThread, index: usize, value: Value) -> RuntimeResult<()> {
    if thread.set_stack_value(index, value) {
        Ok(())
    } else {
        Err(RuntimeError::RegisterOutOfBounds { register: index })
    }
}

#[cfg(test)]
mod tests;
