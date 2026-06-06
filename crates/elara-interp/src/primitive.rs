//! Primitive bytecode execution.

use elara_bytecode::{Instr, Op, Proto, VerifyError, verify_proto};
use elara_core::{LuaFloat, LuaInteger, LuaThread, Table, Value};

/// Result of executing one prototype.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Values and temporary runtime-owned tables produced by primitive execution.
#[derive(Debug)]
pub struct RuntimeOutput {
    /// Returned Lua values.
    pub values: Vec<Value>,
    /// Runtime table storage referenced by table placeholder values.
    pub tables: Vec<Table>,
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
    /// Arithmetic operand was not numeric.
    NonNumericOperand { op: Op },
    /// Call operand was not callable.
    NonCallableValue,
    /// Closure referenced a missing child prototype.
    ChildOutOfBounds { index: usize },
    /// Upvalue read referenced a missing captured value.
    UpvalueOutOfBounds { index: usize },
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
    let mut tables = Vec::new();
    let values = execute_proto_with_upvalues(proto, &[], &[], &mut closures, &mut tables)?;
    Ok(RuntimeOutput { values, tables })
}

fn execute_proto_with_upvalues(
    proto: &Proto,
    upvalues: &[Value],
    varargs: &[Value],
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut Vec<Table>,
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
            Op::Vararg => {
                if let Some(top) = execute_vararg(&mut thread, instr, varargs)? {
                    dynamic_top = top;
                }
            }
            Op::VarargTable => execute_vararg_table(&mut thread, instr, varargs, tables)?,
            Op::Call => {
                if let Some(top) = execute_call(&mut thread, closures, instr, tables)? {
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

fn execute_vararg_table(
    thread: &mut LuaThread,
    instr: Instr,
    varargs: &[Value],
    tables: &mut Vec<Table>,
) -> RuntimeResult<()> {
    let mut table = Table::new();
    for (index, value) in varargs.iter().copied().enumerate() {
        let key =
            LuaInteger::try_from(index + 1).expect("vararg table index must fit in LuaInteger");
        table.raw_set_integer(key, value);
    }

    let table_index = u32::try_from(tables.len()).expect("runtime table index must fit in u32");
    tables.push(table);
    set_register(thread, instr.a().into(), Value::table_index(table_index))
}

fn execute_call(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut Vec<Table>,
) -> RuntimeResult<Option<usize>> {
    let callee = register(thread, instr.a().into())?;
    let closure_index = callee
        .as_closure_index()
        .ok_or(RuntimeError::NonCallableValue)? as usize;
    let closure = closures
        .get(closure_index)
        .cloned()
        .ok_or(RuntimeError::NonCallableValue)?;
    let args = collect_call_args(thread, instr)?;
    let returns =
        execute_proto_with_upvalues(&closure.proto, &closure.upvalues, &args, closures, tables)?;

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
mod tests {
    use elara_bytecode::{Op, ProtoBuilder};
    use elara_core::Value;

    use super::{RuntimeError, execute_proto, execute_proto_with_output};

    #[test]
    fn arithmetic_executes_integer_addition() {
        let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
        let left = builder.add_constant(Value::integer(1));
        let right = builder.add_constant(Value::integer(2));
        builder.emit_abx(Op::LoadK, 0, u64::from(left));
        builder.emit_abx(Op::LoadK, 1, u64::from(right));
        builder.emit_abc(Op::Add, 2, 0, 1);
        builder.emit_abc(Op::Return, 2, 1, 0);

        assert_eq!(
            execute_proto(&builder.finish()),
            Ok(vec![Value::integer(3)])
        );
    }

    #[test]
    fn arithmetic_executes_float_division() {
        let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
        let left = builder.add_constant(Value::integer(7));
        let right = builder.add_constant(Value::integer(2));
        builder.emit_abx(Op::LoadK, 0, u64::from(left));
        builder.emit_abx(Op::LoadK, 1, u64::from(right));
        builder.emit_abc(Op::Div, 2, 0, 1);
        builder.emit_abc(Op::Return, 2, 1, 0);

        assert_eq!(
            execute_proto(&builder.finish()),
            Ok(vec![Value::float(3.5)])
        );
    }

    #[test]
    fn arithmetic_executes_unary_minus() {
        let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
        let value = builder.add_constant(Value::integer(4));
        builder.emit_abx(Op::LoadK, 0, u64::from(value));
        builder.emit_abc(Op::Unm, 1, 0, 0);
        builder.emit_abc(Op::Return, 1, 1, 0);

        assert_eq!(
            execute_proto(&builder.finish()),
            Ok(vec![Value::integer(-4)])
        );
    }

    #[test]
    fn arithmetic_reports_non_numeric_operands() {
        let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
        builder.emit_abc(Op::LoadBool, 0, 1, 0);
        builder.emit_abc(Op::LoadBool, 1, 0, 0);
        builder.emit_abc(Op::Add, 2, 0, 1);
        builder.emit_abc(Op::Return, 2, 1, 0);

        assert_eq!(
            execute_proto(&builder.finish()),
            Err(RuntimeError::NonNumericOperand { op: Op::Add })
        );
    }

    #[test]
    fn locals_move_values_between_registers() {
        let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
        let value = builder.add_constant(Value::integer(9));
        builder.emit_abx(Op::LoadK, 0, u64::from(value));
        builder.emit_abc(Op::Move, 1, 0, 0);
        builder.emit_abc(Op::Return, 1, 1, 0);

        assert_eq!(
            execute_proto(&builder.finish()),
            Ok(vec![Value::integer(9)])
        );
    }

    #[test]
    fn calls_execute_child_proto() {
        let mut child_builder = ProtoBuilder::new().with_signature(1, 0, false);
        let value = child_builder.add_constant(Value::integer(42));
        child_builder.emit_abx(Op::LoadK, 0, u64::from(value));
        child_builder.emit_abc(Op::Return, 0, 1, 0);
        let child = child_builder.finish();

        let mut parent = ProtoBuilder::new().with_signature(1, 0, false);
        let child_index = parent.add_child(child);
        parent.emit_abx(Op::Closure, 0, u64::from(child_index));
        parent.emit_abc(Op::Call, 0, 1, 1);
        parent.emit_abc(Op::Return, 0, 1, 0);

        assert_eq!(
            execute_proto(&parent.finish()),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn calls_report_non_callable_values() {
        let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
        let value = builder.add_constant(Value::integer(1));
        builder.emit_abx(Op::LoadK, 0, u64::from(value));
        builder.emit_abc(Op::Call, 0, 1, 1);

        assert_eq!(
            execute_proto(&builder.finish()),
            Err(RuntimeError::NonCallableValue)
        );
    }

    #[test]
    fn closures_capture_parent_stack_values() {
        let mut child_builder = ProtoBuilder::new().with_signature(2, 0, false);
        child_builder.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
        let one = child_builder.add_constant(Value::integer(1));
        child_builder.emit_abc(Op::GetUpvalue, 0, 0, 0);
        child_builder.emit_abx(Op::LoadK, 1, u64::from(one));
        child_builder.emit_abc(Op::Add, 0, 0, 1);
        child_builder.emit_abc(Op::Return, 0, 1, 0);
        let child = child_builder.finish();

        let mut parent = ProtoBuilder::new().with_signature(2, 0, false);
        let value = parent.add_constant(Value::integer(41));
        parent.emit_abx(Op::LoadK, 0, u64::from(value));
        let child_index = parent.add_child(child);
        parent.emit_abx(Op::Closure, 1, u64::from(child_index));
        parent.emit_abc(Op::Call, 1, 1, 1);
        parent.emit_abc(Op::Return, 1, 1, 0);

        assert_eq!(
            execute_proto(&parent.finish()),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn varargs_pass_call_arguments_to_child_proto() {
        let mut child_builder = ProtoBuilder::new().with_signature(1, 0, true);
        child_builder.emit_abc(Op::Vararg, 0, 1, 0);
        child_builder.emit_abc(Op::Return, 0, 1, 0);
        let child = child_builder.finish();

        let mut parent = ProtoBuilder::new().with_signature(2, 0, false);
        let value = parent.add_constant(Value::integer(42));
        let child_index = parent.add_child(child);
        parent.emit_abx(Op::Closure, 0, u64::from(child_index));
        parent.emit_abx(Op::LoadK, 1, u64::from(value));
        parent.emit_abc(Op::Call, 0, 2, 1);
        parent.emit_abc(Op::Return, 0, 1, 0);

        assert_eq!(
            execute_proto(&parent.finish()),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn varargs_return_multiple_requested_results() {
        let mut child_builder = ProtoBuilder::new().with_signature(2, 0, true);
        child_builder.emit_abc(Op::Vararg, 0, 2, 0);
        child_builder.emit_abc(Op::Return, 0, 2, 0);
        let child = child_builder.finish();

        let mut parent = ProtoBuilder::new().with_signature(3, 0, false);
        let first = parent.add_constant(Value::integer(42));
        let second = parent.add_constant(Value::integer(99));
        let child_index = parent.add_child(child);
        parent.emit_abx(Op::Closure, 0, u64::from(child_index));
        parent.emit_abx(Op::LoadK, 1, u64::from(first));
        parent.emit_abx(Op::LoadK, 2, u64::from(second));
        parent.emit_abc(Op::Call, 0, 3, 2);
        parent.emit_abc(Op::Return, 0, 2, 0);

        assert_eq!(
            execute_proto(&parent.finish()),
            Ok(vec![Value::integer(42), Value::integer(99)])
        );
    }

    #[test]
    fn varargs_return_open_call_results() {
        let mut child_builder = ProtoBuilder::new().with_signature(2, 0, true);
        child_builder.emit_abc(Op::Vararg, 0, 0, 0);
        child_builder.emit_abc(Op::Return, 0, 0, 0);
        let child = child_builder.finish();

        let mut parent = ProtoBuilder::new().with_signature(3, 0, false);
        let first = parent.add_constant(Value::integer(42));
        let second = parent.add_constant(Value::integer(99));
        let child_index = parent.add_child(child);
        parent.emit_abx(Op::Closure, 0, u64::from(child_index));
        parent.emit_abx(Op::LoadK, 1, u64::from(first));
        parent.emit_abx(Op::LoadK, 2, u64::from(second));
        parent.emit_abc(Op::Call, 0, 3, 0);
        parent.emit_abc(Op::Return, 0, 0, 0);

        assert_eq!(
            execute_proto(&parent.finish()),
            Ok(vec![Value::integer(42), Value::integer(99)])
        );
    }

    #[test]
    fn varargs_named_table_contains_arguments() {
        let mut child_builder = ProtoBuilder::new().with_signature(1, 0, true);
        child_builder.emit_abc(Op::VarargTable, 0, 0, 0);
        child_builder.emit_abc(Op::Return, 0, 1, 0);
        let child = child_builder.finish();

        let mut parent = ProtoBuilder::new().with_signature(3, 0, false);
        let first = parent.add_constant(Value::integer(42));
        let second = parent.add_constant(Value::integer(99));
        let child_index = parent.add_child(child);
        parent.emit_abx(Op::Closure, 0, u64::from(child_index));
        parent.emit_abx(Op::LoadK, 1, u64::from(first));
        parent.emit_abx(Op::LoadK, 2, u64::from(second));
        parent.emit_abc(Op::Call, 0, 3, 1);
        parent.emit_abc(Op::Return, 0, 1, 0);

        let output = execute_proto_with_output(&parent.finish()).expect("execution should pass");
        let table_index = output.values[0]
            .as_table_index()
            .expect("expected table placeholder");
        let table = &output.tables[table_index as usize];

        assert_eq!(table.raw_get_integer(1), Value::integer(42));
        assert_eq!(table.raw_get_integer(2), Value::integer(99));
        assert_eq!(table.raw_get_integer(3), Value::nil());
    }
}
