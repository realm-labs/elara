use elara_bytecode::{Instr, Op, ProtoBuilder};
use elara_core::{LuaThread, Table, Value};

use super::{
    RuntimeClosure, RuntimeGlobals, RuntimeNatives, RuntimeStrings, RuntimeTables,
    execute_arithmetic, execute_call, execute_comparison, execute_concat, execute_len,
};

fn constant_closure(value: Value) -> RuntimeClosure {
    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let constant = builder.add_constant(value);
    builder.emit_abx(Op::LoadK, 0, u64::from(constant));
    builder.emit_abc(Op::Return, 0, 1, 0);
    RuntimeClosure {
        proto: builder.finish(),
        upvalues: Vec::new(),
    }
}

fn runtime_globals(tables: &mut RuntimeTables) -> RuntimeGlobals {
    let global_table = tables.push_table(Table::new());
    RuntimeGlobals::new(global_table)
}

#[test]
fn metamethods_arithmetic_calls_left_operand_add() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let add_key = strings.intern_short_value("__add");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(add_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::integer(42))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::integer(1));
    thread.push_value(Value::nil());

    execute_arithmetic(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Add, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__add should execute");

    assert_eq!(thread.stack_value(2), Some(Value::integer(42)));
}

#[test]
fn metamethods_arithmetic_calls_right_operand_add() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let left = tables.push_table(Table::new());
    let right = tables.push_table(Table::new());
    let add_key = strings.intern_short_value("__add");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(add_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(right as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::integer(99))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(left));
    thread.push_value(Value::table_index(right));
    thread.push_value(Value::nil());

    execute_arithmetic(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Add, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__add should execute");

    assert_eq!(thread.stack_value(2), Some(Value::integer(99)));
}

#[test]
fn metamethods_arithmetic_calls_unary_minus() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let unm_key = strings.intern_short_value("__unm");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(unm_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::integer(-7))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::nil());

    execute_arithmetic(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Unm, 1, 0, 0),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__unm should execute");

    assert_eq!(thread.stack_value(1), Some(Value::integer(-7)));
}

#[test]
fn metamethods_comparison_executes_raw_less_than() {
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::integer(1));
    thread.push_value(Value::integer(2));
    thread.push_value(Value::nil());

    execute_comparison(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Lt, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("raw less-than should execute");

    assert_eq!(thread.stack_value(2), Some(Value::boolean(true)));
}

#[test]
fn metamethods_comparison_calls_eq_for_distinct_tables() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let left = tables.push_table(Table::new());
    let right = tables.push_table(Table::new());
    let eq_key = strings.intern_short_value("__eq");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(eq_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(left as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::boolean(true))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(left));
    thread.push_value(Value::table_index(right));
    thread.push_value(Value::nil());

    execute_comparison(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Eq, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__eq should execute");

    assert_eq!(thread.stack_value(2), Some(Value::boolean(true)));
}

#[test]
fn metamethods_comparison_calls_less_than() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let left = tables.push_table(Table::new());
    let right = tables.push_table(Table::new());
    let lt_key = strings.intern_short_value("__lt");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(lt_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(left as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::boolean(true))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(left));
    thread.push_value(Value::table_index(right));
    thread.push_value(Value::nil());

    execute_comparison(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Lt, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__lt should execute");

    assert_eq!(thread.stack_value(2), Some(Value::boolean(true)));
}

#[test]
fn metamethods_len_executes_raw_table_length() {
    let mut closures = Vec::new();
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut table_value = Table::new();
    assert!(table_value.raw_set_integer(1, Value::integer(10)));
    assert!(table_value.raw_set_integer(2, Value::integer(20)));
    let table = tables.push_table(table_value);
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::nil());

    execute_len(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Len, 1, 0, 0),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("raw table length should execute");

    assert_eq!(thread.stack_value(1), Some(Value::integer(2)));
}

#[test]
fn metamethods_len_calls_function_fallback() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let len_key = strings.intern_short_value("__len");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(len_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::integer(77))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::nil());

    execute_len(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Len, 1, 0, 0),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__len should execute");

    assert_eq!(thread.stack_value(1), Some(Value::integer(77)));
}

#[test]
fn metamethods_call_invokes_function_fallback() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let call_key = strings.intern_short_value("__call");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(call_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::integer(123))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::integer(1));

    execute_call(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Call, 0, 2, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__call should execute");

    assert_eq!(thread.stack_value(0), Some(Value::integer(123)));
}

#[test]
fn metamethods_call_invokes_native_function_fallback() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let call_key = strings.intern_short_value("__call");
    let natives = RuntimeNatives::new();
    let native = natives.push(|_context, args| {
        assert!(args.first().copied().is_some_and(Value::is_table));
        Ok(vec![Value::integer(321)])
    });
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(call_key, Value::native_function_index(native)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = Vec::new();
    let mut globals = runtime_globals(&mut tables);
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::integer(1));

    execute_call(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Call, 0, 2, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("native __call should execute");

    assert_eq!(thread.stack_value(0), Some(Value::integer(321)));
}

#[test]
fn metamethods_concat_executes_raw_short_strings() {
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let left = strings.intern_short_value("a");
    let right = strings.intern_short_value("b");
    let mut thread = LuaThread::new();
    thread.push_value(left);
    thread.push_value(right);
    thread.push_value(Value::nil());

    execute_concat(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Concat, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("raw short-string concat should execute");

    let expected = strings.intern_short_value("ab");
    assert_eq!(thread.stack_value(2), Some(expected));
}

#[test]
fn metamethods_concat_calls_function_fallback() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let concat_key = strings.intern_short_value("__concat");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(concat_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::integer(321))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::integer(1));
    thread.push_value(Value::nil());

    execute_concat(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Concat, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__concat should execute");

    assert_eq!(thread.stack_value(2), Some(Value::integer(321)));
}
