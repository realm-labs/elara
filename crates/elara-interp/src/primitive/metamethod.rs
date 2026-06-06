//! Arithmetic and comparison helpers with metamethod fallback.

use elara_bytecode::{Instr, Op};
use elara_core::{LuaFloat, LuaInteger, LuaThread, Value};

use super::{
    RuntimeClosure, RuntimeError, RuntimeResult, RuntimeStrings, RuntimeTables, call_closure,
    is_truthy, register, set_register,
};

pub(super) fn execute_arithmetic(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let op = instr.op();
    if op == Op::Unm {
        let value = register(thread, instr.b() as usize)?;
        let result = if let Some(result) = negate(value) {
            result
        } else {
            call_unary_metamethod(op, value, closures, tables, strings)?
                .ok_or(RuntimeError::NonNumericOperand { op })?
        };
        return set_register(thread, instr.a().into(), result);
    }

    let left = register(thread, instr.b() as usize)?;
    let right = register(thread, instr.c() as usize)?;
    let result = if let Some(result) = binary_arithmetic(op, left, right) {
        result
    } else {
        call_binary_metamethod(op, left, right, closures, tables, strings)?
            .ok_or(RuntimeError::NonNumericOperand { op })?
    };
    set_register(thread, instr.a().into(), result)
}

pub(super) fn execute_comparison(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let op = instr.op();
    let left = register(thread, instr.b() as usize)?;
    let right = register(thread, instr.c() as usize)?;
    let result = match op {
        Op::Eq => equality_comparison(left, right, closures, tables, strings)?,
        Op::Lt | Op::Le => order_comparison(op, left, right, closures, tables, strings)?,
        _ => return Err(RuntimeError::UnsupportedOpcode { op }),
    };
    set_register(thread, instr.a().into(), Value::boolean(result))
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

fn call_binary_metamethod(
    op: Op,
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<Value>> {
    let Some(name) = binary_metamethod_name(op) else {
        return Ok(None);
    };

    let metamethod = match tables.metamethod_for_value(left, name, strings)? {
        Some(metamethod) => Some(metamethod),
        None => tables.metamethod_for_value(right, name, strings)?,
    };
    let Some(metamethod) = metamethod else {
        return Ok(None);
    };
    let Some(closure) = metamethod.as_closure_index() else {
        return Err(RuntimeError::UnsupportedMetamethod { name });
    };

    let returns = call_closure(closures, closure as usize, &[left, right], tables, strings)?;
    Ok(Some(returns.first().copied().unwrap_or_else(Value::nil)))
}

fn call_unary_metamethod(
    op: Op,
    value: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<Value>> {
    let Some(name) = unary_metamethod_name(op) else {
        return Ok(None);
    };
    let Some(metamethod) = tables.metamethod_for_value(value, name, strings)? else {
        return Ok(None);
    };
    let Some(closure) = metamethod.as_closure_index() else {
        return Err(RuntimeError::UnsupportedMetamethod { name });
    };

    let returns = call_closure(closures, closure as usize, &[value], tables, strings)?;
    Ok(Some(returns.first().copied().unwrap_or_else(Value::nil)))
}

fn binary_metamethod_name(op: Op) -> Option<&'static str> {
    match op {
        Op::Add => Some("__add"),
        Op::Sub => Some("__sub"),
        Op::Mul => Some("__mul"),
        Op::Div => Some("__div"),
        Op::IDiv => Some("__idiv"),
        Op::Mod => Some("__mod"),
        Op::Pow => Some("__pow"),
        _ => None,
    }
}

fn unary_metamethod_name(op: Op) -> Option<&'static str> {
    match op {
        Op::Unm => Some("__unm"),
        _ => None,
    }
}

fn equality_comparison(
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<bool> {
    if left == right {
        return Ok(true);
    }
    if left.tag() != right.tag() {
        return Ok(false);
    }
    let Some(result) = call_comparison_metamethod("__eq", left, right, closures, tables, strings)?
    else {
        return Ok(false);
    };
    Ok(is_truthy(result))
}

fn order_comparison(
    op: Op,
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<bool> {
    if let Some(result) = raw_order_comparison(op, left, right) {
        return Ok(result);
    }

    let name = match op {
        Op::Lt => "__lt",
        Op::Le => "__le",
        _ => return Err(RuntimeError::UnsupportedOpcode { op }),
    };
    let Some(result) = call_comparison_metamethod(name, left, right, closures, tables, strings)?
    else {
        return Err(RuntimeError::NonComparableOperand { op });
    };
    Ok(is_truthy(result))
}

fn raw_order_comparison(op: Op, left: Value, right: Value) -> Option<bool> {
    if let (Some(left), Some(right)) = (left.as_integer(), right.as_integer()) {
        return match op {
            Op::Lt => Some(left < right),
            Op::Le => Some(left <= right),
            _ => None,
        };
    }

    let left = left.to_float()?;
    let right = right.to_float()?;
    match op {
        Op::Lt => Some(left < right),
        Op::Le => Some(left <= right),
        _ => None,
    }
}

fn call_comparison_metamethod(
    name: &'static str,
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<Value>> {
    let metamethod = match tables.metamethod_for_value(left, name, strings)? {
        Some(metamethod) => Some(metamethod),
        None => tables.metamethod_for_value(right, name, strings)?,
    };
    let Some(metamethod) = metamethod else {
        return Ok(None);
    };
    let Some(closure) = metamethod.as_closure_index() else {
        return Err(RuntimeError::UnsupportedMetamethod { name });
    };

    let returns = call_closure(closures, closure as usize, &[left, right], tables, strings)?;
    Ok(Some(returns.first().copied().unwrap_or_else(Value::nil)))
}
