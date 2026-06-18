use elara_bytecode::{Instr, Op, ProtoBuilder};
use elara_core::{LuaThread, SHORT_STRING_MAX_BYTES, Table, Value};

use super::{
    RuntimeClosure, RuntimeGlobals, RuntimeNatives, RuntimeStrings, RuntimeTables, execute_add_int,
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
fn superinstruction_add_int_calls_left_operand_add() {
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
    let mut closures = vec![constant_closure(Value::integer(77))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::nil());

    execute_add_int(
        &mut thread,
        &mut closures,
        Instr::abc(Op::AddInt, 1, 0, 5),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__add should execute");

    assert_eq!(thread.stack_value(1), Some(Value::integer(77)));
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
fn metamethods_arithmetic_calls_bitwise_and() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let band_key = strings.intern_short_value("__band");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(band_key, Value::closure_index(0)));
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
    thread.push_value(Value::nil());

    execute_arithmetic(
        &mut thread,
        &mut closures,
        Instr::abc(Op::BAnd, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__band should execute");

    assert_eq!(thread.stack_value(2), Some(Value::integer(123)));
}

#[test]
fn metamethods_arithmetic_calls_bitwise_not() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let bnot_key = strings.intern_short_value("__bnot");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(bnot_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut closures = vec![constant_closure(Value::integer(321))];
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::nil());

    execute_arithmetic(
        &mut thread,
        &mut closures,
        Instr::abc(Op::BNot, 1, 0, 0),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("__bnot should execute");

    assert_eq!(thread.stack_value(1), Some(Value::integer(321)));
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
fn metamethods_comparison_executes_raw_string_less_than() {
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let left = strings.intern_value("alpha");
    let right = strings.intern_value("beta");
    let mut thread = LuaThread::new();
    thread.push_value(left);
    thread.push_value(right);
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
    .expect("raw string less-than should execute");

    assert_eq!(thread.stack_value(2), Some(Value::boolean(true)));
}

#[test]
fn metamethods_comparison_executes_raw_string_less_equal() {
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let left = strings.intern_value("same");
    let right = strings.intern_value("same");
    let mut thread = LuaThread::new();
    thread.push_value(left);
    thread.push_value(right);
    thread.push_value(Value::nil());

    execute_comparison(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Le, 2, 0, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("raw string less-or-equal should execute");

    assert_eq!(thread.stack_value(2), Some(Value::boolean(true)));
}

#[test]
fn metamethods_comparison_executes_raw_long_string_equality() {
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let bytes = vec![b'a'; SHORT_STRING_MAX_BYTES + 1];
    let left = strings.intern_value(&bytes);
    let right = strings.intern_value(&bytes);
    let mut thread = LuaThread::new();
    thread.push_value(left);
    thread.push_value(right);
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
    .expect("raw long string equality should execute");

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
fn metamethods_len_executes_raw_string_length() {
    let mut closures = Vec::new();
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut thread = LuaThread::new();
    thread.push_value(strings.intern_value("abcd"));
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
    .expect("raw string length should execute");

    assert_eq!(thread.stack_value(1), Some(Value::integer(4)));
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
fn metamethods_call_chains_callable_fallbacks() {
    let mut strings = RuntimeStrings::new();
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let proxy = tables.push_table(Table::new());
    let call_key = strings.intern_short_value("__call");
    let natives = RuntimeNatives::new();
    let native = natives.push(move |_context, args| {
        assert_eq!(args.first().copied(), Some(Value::table_index(proxy)));
        assert_eq!(args.get(1).copied(), Some(Value::table_index(table)));
        assert_eq!(args.get(2).copied(), Some(Value::integer(7)));
        Ok(vec![Value::integer(654)])
    });

    let mut table_metatable = Table::new();
    assert!(table_metatable.raw_set_value(call_key, Value::table_index(proxy)));
    let table_metatable = tables.push_table(table_metatable);
    tables
        .set_metatable(table as usize, Some(table_metatable))
        .expect("metatable link should be valid");

    let mut proxy_metatable = Table::new();
    assert!(proxy_metatable.raw_set_value(call_key, Value::native_function_index(native)));
    let proxy_metatable = tables.push_table(proxy_metatable);
    tables
        .set_metatable(proxy as usize, Some(proxy_metatable))
        .expect("proxy metatable link should be valid");

    let mut closures = Vec::new();
    let mut globals = runtime_globals(&mut tables);
    let mut thread = LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::integer(7));

    execute_call(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Call, 0, 2, 1),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .expect("chained __call should execute");

    assert_eq!(thread.stack_value(0), Some(Value::integer(654)));
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
fn metamethods_concat_executes_raw_runtime_strings() {
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let left = strings.intern_value("a".repeat(41));
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
    .expect("raw runtime string concat should execute");

    let value = thread
        .stack_value(2)
        .expect("concat result should be written");
    let expected = "a".repeat(41) + "b";
    assert_eq!(strings.short_string_bytes(value), None);
    assert_eq!(strings.string_bytes(value), Some(expected.as_bytes()));
}

#[test]
fn metamethods_concat_coerces_numeric_operands() {
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let right = strings.intern_short_value(" apples");
    let mut thread = LuaThread::new();
    thread.push_value(Value::integer(12));
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
    .expect("raw numeric concat should execute");

    let value = thread
        .stack_value(2)
        .expect("concat result should be written");
    assert_eq!(strings.string_bytes(value), Some(b"12 apples".as_slice()));
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
