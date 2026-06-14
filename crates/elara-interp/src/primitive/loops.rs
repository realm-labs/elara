//! Loop opcode helpers for the primitive interpreter.

use elara_bytecode::Instr;
use elara_core::{LuaFloat, LuaInteger, LuaThread, Value};

use super::{
    RuntimeClosure, RuntimeErrorKind, RuntimeGlobals, RuntimeNatives, RuntimeResult,
    RuntimeStrings, RuntimeTables, call_function, callable_from_value, register, set_register,
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

    let init = init.to_float().ok_or(RuntimeErrorKind::ForLoopNonNumeric {
        operand: "initial value",
    })?;
    let limit = limit
        .to_float()
        .ok_or(RuntimeErrorKind::ForLoopNonNumeric { operand: "limit" })?;
    let step = step
        .to_float()
        .ok_or(RuntimeErrorKind::ForLoopNonNumeric { operand: "step" })?;
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
        return Err(RuntimeErrorKind::ForLoopStepZero.into());
    }
    if should_skip_integer_for(init, limit, step) {
        return Ok(true);
    }

    let count = if step > 0 {
        (i128::from(limit) - i128::from(init)) / i128::from(step)
    } else {
        (i128::from(init) - i128::from(limit)) / -i128::from(step)
    };
    let count = LuaInteger::try_from(count).map_err(|_| RuntimeErrorKind::ForLoopCountOverflow)?;

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
        return Err(RuntimeErrorKind::ForLoopStepZero.into());
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
        .ok_or(RuntimeErrorKind::ForLoopNonNumeric { operand: "step" })?;
    let index = register(thread, base + 2)?
        .as_integer()
        .ok_or(RuntimeErrorKind::ForLoopNonNumeric {
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
        .ok_or(RuntimeErrorKind::ForLoopNonNumeric { operand: "limit" })?;
    let step = register(thread, base + 1)?
        .as_float()
        .ok_or(RuntimeErrorKind::ForLoopNonNumeric { operand: "step" })?;
    let index =
        register(thread, base + 2)?
            .as_float()
            .ok_or(RuntimeErrorKind::ForLoopNonNumeric {
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
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let base = usize::from(instr.a());
    let iterator = register(thread, base)?;
    let state = register(thread, base + 1)?;
    let control = register(thread, base + 2)?;
    let args = vec![state, control];
    let callable = callable_from_value(iterator, args, tables, strings)?;
    let mut to_be_closed = Vec::new();
    let mut debug_frames = Vec::new();
    let mut context = super::ExecutionContext {
        closures,
        tables,
        strings,
        natives,
        globals,
        to_be_closed: &mut to_be_closed,
        debug_frames: &mut debug_frames,
    };
    let returns = call_function(callable, &mut context, Some(thread), None)?;

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
