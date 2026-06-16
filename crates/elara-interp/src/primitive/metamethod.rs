//! Arithmetic and comparison helpers with metamethod fallback.

use elara_bytecode::{Instr, Op};
use elara_core::{LuaFloat, LuaInteger, LuaThread, Value};

use super::{
    RuntimeClosure, RuntimeErrorKind, RuntimeGlobals, RuntimeNatives, RuntimeResult,
    RuntimeStrings, RuntimeTables, call_closure, is_truthy, register, set_register,
};

pub(super) fn execute_arithmetic(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let op = instr.op();
    if matches!(op, Op::Unm | Op::BNot) {
        let value = register(thread, instr.b() as usize)?;
        let result = if let Some(result) = unary_arithmetic(op, value) {
            result
        } else {
            call_unary_metamethod(op, value, closures, tables, strings, natives, globals)?
                .ok_or(RuntimeErrorKind::NonNumericOperand { op })?
        };
        return set_register(thread, instr.a().into(), result);
    }

    let left = register(thread, instr.b() as usize)?;
    let right = register(thread, instr.c() as usize)?;
    let result = if let Some(result) = binary_arithmetic(op, left, right) {
        result
    } else {
        call_binary_metamethod(op, left, right, closures, tables, strings, natives, globals)?
            .ok_or(RuntimeErrorKind::NonNumericOperand { op })?
    };
    set_register(thread, instr.a().into(), result)
}

pub(super) fn execute_add_int(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let left = register(thread, instr.b() as usize)?;
    let right = Value::integer(LuaInteger::from(instr.c()));
    let result = if let Some(result) = binary_arithmetic(Op::Add, left, right) {
        result
    } else {
        call_binary_metamethod(
            Op::Add,
            left,
            right,
            closures,
            tables,
            strings,
            natives,
            globals,
        )?
        .ok_or(RuntimeErrorKind::NonNumericOperand { op: Op::Add })?
    };
    set_register(thread, instr.a().into(), result)
}

pub(super) fn execute_comparison(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let op = instr.op();
    let left = register(thread, instr.b() as usize)?;
    let right = register(thread, instr.c() as usize)?;
    let result = match op {
        Op::Eq => equality_comparison(left, right, closures, tables, strings, natives, globals)?,
        Op::Lt | Op::Le => {
            order_comparison(op, left, right, closures, tables, strings, natives, globals)?
        }
        _ => return Err(RuntimeErrorKind::UnsupportedOpcode { op }.into()),
    };
    set_register(thread, instr.a().into(), Value::boolean(result))
}

pub(super) fn execute_len(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let value = register(thread, instr.b() as usize)?;
    let result = if let Some(result) =
        call_named_unary_metamethod("__len", value, closures, tables, strings, natives, globals)?
    {
        result
    } else if let Some(table_index) = value.as_table_index() {
        let table = tables
            .get(table_index as usize)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        Value::integer(
            LuaInteger::try_from(table.array_len()).expect("table length must fit in LuaInteger"),
        )
    } else if let Some(bytes) = strings.string_bytes(value) {
        Value::integer(
            LuaInteger::try_from(bytes.len()).expect("string length must fit in LuaInteger"),
        )
    } else {
        return Err(RuntimeErrorKind::NonLengthOperand.into());
    };
    set_register(thread, instr.a().into(), result)
}

pub(super) fn execute_concat(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let left = register(thread, instr.b() as usize)?;
    let right = register(thread, instr.c() as usize)?;
    let result = if let Some(result) = raw_concat(left, right, strings)? {
        result
    } else {
        call_named_binary_metamethod(
            "__concat", left, right, closures, tables, strings, natives, globals,
        )?
        .ok_or(RuntimeErrorKind::NonConcatOperand)?
    };
    set_register(thread, instr.a().into(), result)
}

fn binary_arithmetic(op: Op, left: Value, right: Value) -> Option<Value> {
    if matches!(op, Op::BAnd | Op::BOr | Op::BXor | Op::Shl | Op::Shr) {
        return bitwise_arithmetic(op, left, right);
    }

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

fn bitwise_arithmetic(op: Op, left: Value, right: Value) -> Option<Value> {
    let left = left.to_integer_exact()?;
    let right = right.to_integer_exact()?;
    let value = match op {
        Op::BAnd => left & right,
        Op::BOr => left | right,
        Op::BXor => left ^ right,
        Op::Shl => shift_left(left, right),
        Op::Shr => shift_left(left, right.wrapping_neg()),
        _ => return None,
    };
    Some(Value::integer(value))
}

fn unary_arithmetic(op: Op, value: Value) -> Option<Value> {
    match op {
        Op::Unm => {
            if let Some(value) = value.as_integer() {
                return value.checked_neg().map(Value::integer);
            }
            Some(Value::float(-value.to_float()?))
        }
        Op::BNot => value.to_integer_exact().map(|value| Value::integer(!value)),
        _ => None,
    }
}

fn shift_left(value: LuaInteger, count: LuaInteger) -> LuaInteger {
    let bits = LuaInteger::BITS as LuaInteger;
    if count <= -bits || count >= bits {
        0
    } else if count < 0 {
        value >> u32::try_from(count.unsigned_abs()).expect("shift count is in range")
    } else {
        value << u32::try_from(count).expect("shift count is in range")
    }
}

#[allow(clippy::too_many_arguments)]
fn call_binary_metamethod(
    op: Op,
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<Value>> {
    let Some(name) = binary_metamethod_name(op) else {
        return Ok(None);
    };
    call_named_binary_metamethod(
        name, left, right, closures, tables, strings, natives, globals,
    )
}

#[allow(clippy::too_many_arguments)]
fn call_named_binary_metamethod(
    name: &'static str,
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<Value>> {
    let metamethod = match tables.metamethod_for_value(left, name, strings)? {
        Some(metamethod) => Some(metamethod),
        None => tables.metamethod_for_value(right, name, strings)?,
    };
    let Some(metamethod) = metamethod else {
        return Ok(None);
    };
    let Some(closure) = metamethod.as_closure_index() else {
        return Err(RuntimeErrorKind::UnsupportedMetamethod { name }.into());
    };

    let returns = call_closure(
        closures,
        closure as usize,
        &[left, right],
        tables,
        strings,
        natives,
        globals,
        None,
    )?;
    Ok(Some(returns.first().copied().unwrap_or_else(Value::nil)))
}

fn raw_concat(
    left: Value,
    right: Value,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<Value>> {
    let bytes = {
        let Some(left) = strings.string_bytes(left) else {
            return Ok(None);
        };
        let Some(right) = strings.string_bytes(right) else {
            return Ok(None);
        };
        let len = left
            .len()
            .checked_add(right.len())
            .ok_or(RuntimeErrorKind::StringConcatTooLong)?;
        let mut bytes = Vec::with_capacity(len);
        bytes.extend_from_slice(left);
        bytes.extend_from_slice(right);
        bytes
    };
    Ok(Some(strings.intern_value(bytes)))
}

fn call_unary_metamethod(
    op: Op,
    value: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<Value>> {
    let Some(name) = unary_metamethod_name(op) else {
        return Ok(None);
    };
    let Some(metamethod) = tables.metamethod_for_value(value, name, strings)? else {
        return Ok(None);
    };
    call_unary_closure_metamethod(
        name, metamethod, value, closures, tables, strings, natives, globals,
    )
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
        Op::BAnd => Some("__band"),
        Op::BOr => Some("__bor"),
        Op::BXor => Some("__bxor"),
        Op::Shl => Some("__shl"),
        Op::Shr => Some("__shr"),
        _ => None,
    }
}

fn unary_metamethod_name(op: Op) -> Option<&'static str> {
    match op {
        Op::Unm => Some("__unm"),
        Op::BNot => Some("__bnot"),
        _ => None,
    }
}

fn call_named_unary_metamethod(
    name: &'static str,
    value: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<Value>> {
    let Some(metamethod) = tables.metamethod_for_value(value, name, strings)? else {
        return Ok(None);
    };
    call_unary_closure_metamethod(
        name, metamethod, value, closures, tables, strings, natives, globals,
    )
}

#[allow(clippy::too_many_arguments)]
fn call_unary_closure_metamethod(
    name: &'static str,
    metamethod: Value,
    value: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<Value>> {
    let Some(closure) = metamethod.as_closure_index() else {
        return Err(RuntimeErrorKind::UnsupportedMetamethod { name }.into());
    };

    let returns = call_closure(
        closures,
        closure as usize,
        &[value],
        tables,
        strings,
        natives,
        globals,
        None,
    )?;
    Ok(Some(returns.first().copied().unwrap_or_else(Value::nil)))
}

fn equality_comparison(
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<bool> {
    if left == right {
        return Ok(true);
    }
    if left.tag() != right.tag() {
        return Ok(false);
    }
    let Some(result) = call_comparison_metamethod(
        "__eq", left, right, closures, tables, strings, natives, globals,
    )?
    else {
        return Ok(false);
    };
    Ok(is_truthy(result))
}

#[allow(clippy::too_many_arguments)]
fn order_comparison(
    op: Op,
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<bool> {
    if let Some(result) = raw_order_comparison(op, left, right) {
        return Ok(result);
    }

    let name = match op {
        Op::Lt => "__lt",
        Op::Le => "__le",
        _ => return Err(RuntimeErrorKind::UnsupportedOpcode { op }.into()),
    };
    let Some(result) = call_comparison_metamethod(
        name, left, right, closures, tables, strings, natives, globals,
    )?
    else {
        return Err(RuntimeErrorKind::NonComparableOperand { op }.into());
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

#[allow(clippy::too_many_arguments)]
fn call_comparison_metamethod(
    name: &'static str,
    left: Value,
    right: Value,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<Value>> {
    let metamethod = match tables.metamethod_for_value(left, name, strings)? {
        Some(metamethod) => Some(metamethod),
        None => tables.metamethod_for_value(right, name, strings)?,
    };
    let Some(metamethod) = metamethod else {
        return Ok(None);
    };
    let Some(closure) = metamethod.as_closure_index() else {
        return Err(RuntimeErrorKind::UnsupportedMetamethod { name }.into());
    };

    let returns = call_closure(
        closures,
        closure as usize,
        &[left, right],
        tables,
        strings,
        natives,
        globals,
        None,
    )?;
    Ok(Some(returns.first().copied().unwrap_or_else(Value::nil)))
}
