//! Loop opcode helpers for the primitive interpreter.

use elara_bytecode::Instr;
use elara_core::{LuaFloat, LuaInteger, LuaThread, Value};

use super::{
    RuntimeClosure, RuntimeError, RuntimeResult, RuntimeStrings, RuntimeTables, call_closure,
    register, set_register,
};

pub(super) fn prepare_numeric_for(thread: &mut LuaThread, instr: Instr) -> RuntimeResult<bool> {
    let base = usize::from(instr.a());
    let init = register(thread, base)?;
    let limit = register(thread, base + 1)?;
    let step = register(thread, base + 2)?;

    if let (Some(init), Some(limit), Some(step)) = (
        init.as_integer(),
        limit.to_integer_exact(),
        step.as_integer(),
    ) {
        return prepare_integer_for(thread, base, init, limit, step);
    }

    let init = init.to_float().ok_or(RuntimeError::ForLoopNonNumeric {
        operand: "initial value",
    })?;
    let limit = limit
        .to_float()
        .ok_or(RuntimeError::ForLoopNonNumeric { operand: "limit" })?;
    let step = step
        .to_float()
        .ok_or(RuntimeError::ForLoopNonNumeric { operand: "step" })?;
    prepare_float_for(thread, base, init, limit, step)
}

fn prepare_integer_for(
    thread: &mut LuaThread,
    base: usize,
    init: LuaInteger,
    limit: LuaInteger,
    step: LuaInteger,
) -> RuntimeResult<bool> {
    if step == 0 {
        return Err(RuntimeError::ForLoopStepZero);
    }
    if should_skip_integer_for(init, limit, step) {
        return Ok(true);
    }

    let count = if step > 0 {
        (i128::from(limit) - i128::from(init)) / i128::from(step)
    } else {
        (i128::from(init) - i128::from(limit)) / -i128::from(step)
    };
    let count = LuaInteger::try_from(count).map_err(|_| RuntimeError::ForLoopCountOverflow)?;

    set_register(thread, base, Value::integer(count))?;
    set_register(thread, base + 1, Value::integer(step))?;
    set_register(thread, base + 2, Value::integer(init))?;
    Ok(false)
}

fn prepare_float_for(
    thread: &mut LuaThread,
    base: usize,
    init: LuaFloat,
    limit: LuaFloat,
    step: LuaFloat,
) -> RuntimeResult<bool> {
    if step == 0.0 {
        return Err(RuntimeError::ForLoopStepZero);
    }
    if should_skip_numeric_for(init, limit, step) {
        return Ok(true);
    }

    set_register(thread, base, Value::float(limit))?;
    set_register(thread, base + 1, Value::float(step))?;
    set_register(thread, base + 2, Value::float(init))?;
    Ok(false)
}

fn should_skip_numeric_for(init: LuaFloat, limit: LuaFloat, step: LuaFloat) -> bool {
    if step > 0.0 {
        init > limit
    } else {
        init < limit
    }
}

fn should_skip_integer_for(init: LuaInteger, limit: LuaInteger, step: LuaInteger) -> bool {
    if step > 0 { init > limit } else { init < limit }
}

pub(super) fn execute_numeric_for_loop(
    thread: &mut LuaThread,
    instr: Instr,
) -> RuntimeResult<bool> {
    let base = usize::from(instr.a());
    let state = register(thread, base)?;
    if let Some(count) = state.as_integer() {
        return execute_integer_for_loop(thread, base, count);
    }
    execute_float_for_loop(thread, base)
}

fn execute_integer_for_loop(
    thread: &mut LuaThread,
    base: usize,
    count: LuaInteger,
) -> RuntimeResult<bool> {
    if count <= 0 {
        return Ok(false);
    }

    let step = register(thread, base + 1)?
        .as_integer()
        .ok_or(RuntimeError::ForLoopNonNumeric { operand: "step" })?;
    let index = register(thread, base + 2)?
        .as_integer()
        .ok_or(RuntimeError::ForLoopNonNumeric {
            operand: "initial value",
        })?
        .wrapping_add(step);
    set_register(thread, base, Value::integer(count - 1))?;
    set_register(thread, base + 2, Value::integer(index))?;
    Ok(true)
}

fn execute_float_for_loop(thread: &mut LuaThread, base: usize) -> RuntimeResult<bool> {
    let limit = register(thread, base)?
        .as_float()
        .ok_or(RuntimeError::ForLoopNonNumeric { operand: "limit" })?;
    let step = register(thread, base + 1)?
        .as_float()
        .ok_or(RuntimeError::ForLoopNonNumeric { operand: "step" })?;
    let index = register(thread, base + 2)?
        .as_float()
        .ok_or(RuntimeError::ForLoopNonNumeric {
            operand: "initial value",
        })?
        + step;

    let continues = if step > 0.0 {
        index <= limit
    } else {
        index >= limit
    };
    if continues {
        set_register(thread, base + 2, Value::float(index))?;
    }
    Ok(continues)
}

pub(super) fn execute_generic_for_call(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let base = usize::from(instr.a());
    let iterator = register(thread, base)?
        .as_closure_index()
        .ok_or(RuntimeError::NonCallableValue)? as usize;
    let state = register(thread, base + 1)?;
    let control = register(thread, base + 2)?;
    let returns = call_closure(closures, iterator, &[state, control], tables, strings)?;

    for index in 0..instr.c() as usize {
        let value = returns.get(index).copied().unwrap_or_else(Value::nil);
        set_register(thread, base + 3 + index, value)?;
    }
    Ok(())
}

pub(super) fn execute_generic_for_loop(
    thread: &mut LuaThread,
    instr: Instr,
) -> RuntimeResult<bool> {
    let base = usize::from(instr.a());
    let first = register(thread, base + 3)?;
    if first.is_nil() {
        return Ok(false);
    }

    set_register(thread, base + 2, first)?;
    Ok(true)
}
