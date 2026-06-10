use elara_bytecode::{Instr, Op, ProtoBuilder};
use elara_core::{Table, ThreadStatus, Value};

use super::{
    CoroutineFrame, CoroutineResume, ExecutionContext, PrimitiveCoroutine, ProtectedRuntimeOutput,
    RuntimeEnvironment, RuntimeErrorKind, RuntimeGlobals, RuntimeNatives, RuntimeStrings,
    RuntimeTables, close_to_base, execute_proto, execute_proto_protected,
    execute_proto_with_environment, execute_proto_with_natives, execute_proto_with_output,
    execute_tbc,
};

fn assert_runtime_error_kind(
    result: Result<Vec<Value>, super::RuntimeError>,
    expected: RuntimeErrorKind,
) {
    let error = result.expect_err("expected runtime error");
    assert_eq!(error.kind(), &expected);
}

fn runtime_globals(tables: &mut RuntimeTables) -> RuntimeGlobals {
    let global_table = tables.push_table(Table::new());
    RuntimeGlobals::new(global_table)
}

fn native_add(args: &[Value]) -> super::RuntimeResult<Vec<Value>> {
    let left = args
        .first()
        .and_then(|value| value.as_integer())
        .ok_or(RuntimeErrorKind::NonNumericOperand { op: Op::Add })?;
    let right = args
        .get(1)
        .and_then(|value| value.as_integer())
        .ok_or(RuntimeErrorKind::NonNumericOperand { op: Op::Add })?;
    Ok(vec![Value::integer(left + right)])
}

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
fn native_functions_execute_call() {
    let mut natives = RuntimeNatives::new();
    let native = natives.push_simple(native_add);

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let callee = builder.add_constant(Value::native_function_index(native));
    let left = builder.add_constant(Value::integer(20));
    let right = builder.add_constant(Value::integer(22));
    builder.emit_abx(Op::LoadK, 0, u64::from(callee));
    builder.emit_abx(Op::LoadK, 1, u64::from(left));
    builder.emit_abx(Op::LoadK, 2, u64::from(right));
    builder.emit_abc(Op::Call, 0, 3, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let output =
        execute_proto_with_natives(&builder.finish(), natives).expect("native call should pass");
    assert_eq!(output.values, vec![Value::integer(42)]);
}

#[test]
fn native_functions_reject_missing_registry_entry() {
    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let callee = builder.add_constant(Value::native_function_index(99));
    builder.emit_abx(Op::LoadK, 0, u64::from(callee));
    builder.emit_abc(Op::Call, 0, 1, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let result = execute_proto_with_natives(&builder.finish(), RuntimeNatives::new());
    let error = match result {
        Ok(_) => panic!("missing native should error"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind(),
        &RuntimeErrorKind::NativeFunctionOutOfBounds { index: 99 }
    );
}

#[test]
fn native_functions_register_as_initial_globals() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_simple_native_global("add", native_add);

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let name = builder.add_string_constant("add");
    let left = builder.add_constant(Value::integer(20));
    let right = builder.add_constant(Value::integer(22));
    builder.emit_abx(Op::GetEnv, 0, u64::from(name));
    builder.emit_abx(Op::LoadK, 1, u64::from(left));
    builder.emit_abx(Op::LoadK, 2, u64::from(right));
    builder.emit_abc(Op::Call, 0, 3, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("registered native global should execute");
    assert_eq!(output.values, vec![Value::integer(42)]);
}

#[test]
fn native_functions_can_capture_host_state() {
    let mut natives = RuntimeNatives::new();
    let offset = 5;
    let native = natives.push_simple(move |args: &[Value]| {
        let mut values = native_add(args)?;
        values[0] = Value::integer(
            values[0]
                .as_integer()
                .expect("native_add returns an integer")
                + offset,
        );
        Ok(values)
    });

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let callee = builder.add_constant(Value::native_function_index(native));
    let left = builder.add_constant(Value::integer(20));
    let right = builder.add_constant(Value::integer(22));
    builder.emit_abx(Op::LoadK, 0, u64::from(callee));
    builder.emit_abx(Op::LoadK, 1, u64::from(left));
    builder.emit_abx(Op::LoadK, 2, u64::from(right));
    builder.emit_abc(Op::Call, 0, 3, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let output =
        execute_proto_with_natives(&builder.finish(), natives).expect("native call should pass");
    assert_eq!(output.values, vec![Value::integer(47)]);
}

#[test]
fn native_functions_register_inside_initial_global_tables() {
    let mut environment = RuntimeEnvironment::new();
    let native = environment.push_simple_native(native_add);
    environment.set_global_table("math", [("add", Value::native_function_index(native))]);

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let module = builder.add_string_constant("math");
    let field = builder.add_string_constant("add");
    let left = builder.add_constant(Value::integer(20));
    let right = builder.add_constant(Value::integer(22));
    builder.emit_abx(Op::GetEnv, 0, u64::from(module));
    builder.emit_abx(Op::LoadString, 1, u64::from(field));
    builder.emit_abc(Op::GetTable, 0, 0, 1);
    builder.emit_abx(Op::LoadK, 1, u64::from(left));
    builder.emit_abx(Op::LoadK, 2, u64::from(right));
    builder.emit_abc(Op::Call, 0, 3, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("registered native table field should execute");
    assert_eq!(output.values, vec![Value::integer(42)]);
}

#[test]
fn native_functions_can_allocate_runtime_strings() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("label", |context, _args| {
        Ok(vec![context.intern_short_string("ok")?])
    });

    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let name = builder.add_string_constant("label");
    builder.emit_abx(Op::GetEnv, 0, u64::from(name));
    builder.emit_abc(Op::Call, 0, 1, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let mut output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("native string allocation should execute");
    let value = output.values.pop().expect("native should return a value");
    assert_eq!(
        output.strings.short_string_bytes(value),
        Some(b"ok".as_slice())
    );
}

#[test]
fn arithmetic_reports_non_numeric_operands() {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    builder.emit_abc(Op::LoadBool, 0, 1, 0);
    builder.emit_abc(Op::LoadBool, 1, 0, 0);
    builder.emit_abc(Op::Add, 2, 0, 1);
    builder.emit_abc(Op::Return, 2, 1, 0);

    assert_runtime_error_kind(
        execute_proto(&builder.finish()),
        RuntimeErrorKind::NonNumericOperand { op: Op::Add },
    );
}

#[test]
fn protected_execution_returns_successful_values() {
    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.emit_abc(Op::Return, 0, 1, 0);

    match execute_proto_protected(&builder.finish()) {
        ProtectedRuntimeOutput::Ok(output) => assert_eq!(output.values, vec![Value::integer(42)]),
        ProtectedRuntimeOutput::Err(error) => panic!("expected protected success, got {error:?}"),
    }
}

#[test]
fn protected_execution_catches_runtime_errors() {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    builder.emit_abc(Op::LoadBool, 0, 1, 0);
    builder.emit_abc(Op::LoadBool, 1, 0, 0);
    builder.emit_abc(Op::Add, 2, 0, 1);

    match execute_proto_protected(&builder.finish()) {
        ProtectedRuntimeOutput::Ok(output) => {
            panic!("expected protected error, got {:?}", output.values)
        }
        ProtectedRuntimeOutput::Err(error) => {
            assert_eq!(
                error.kind(),
                &RuntimeErrorKind::NonNumericOperand { op: Op::Add }
            );
        }
    }
}

#[test]
fn coroutine_yields_and_resumes_to_return() {
    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let yielded = builder.add_constant(Value::integer(7));
    let returned = builder.add_constant(Value::integer(9));
    builder.emit_abx(Op::LoadK, 0, u64::from(yielded));
    builder.emit_abc(Op::Yield, 0, 1, 0);
    builder.emit_abx(Op::LoadK, 1, u64::from(returned));
    builder.emit_abc(Op::Return, 1, 1, 0);

    let mut coroutine =
        PrimitiveCoroutine::new(builder.finish()).expect("coroutine should be created");

    assert_eq!(coroutine.status(), ThreadStatus::Runnable);
    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Yield(vec![Value::integer(7)])
    );
    assert_eq!(coroutine.status(), ThreadStatus::Suspended);
    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Return(vec![Value::integer(9)])
    );
    assert_eq!(coroutine.status(), ThreadStatus::Dead);
}

#[test]
fn coroutine_resume_arguments_replace_yield_results() {
    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let yielded = builder.add_constant(Value::integer(1));
    builder.emit_abx(Op::LoadK, 0, u64::from(yielded));
    builder.emit_abc(Op::Yield, 0, 1, 0);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let mut coroutine =
        PrimitiveCoroutine::new(builder.finish()).expect("coroutine should be created");

    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Yield(vec![Value::integer(1)])
    );
    assert_eq!(
        coroutine.resume(&[Value::integer(42)]),
        CoroutineResume::Return(vec![Value::integer(42)])
    );
}

#[test]
fn coroutine_yields_from_called_lua_frame() {
    let mut child_builder = ProtoBuilder::new().with_signature(2, 0, false);
    let yielded = child_builder.add_constant(Value::integer(3));
    child_builder.emit_abx(Op::LoadK, 0, u64::from(yielded));
    child_builder.emit_abc(Op::Yield, 0, 1, 0);
    let returned = child_builder.add_constant(Value::integer(5));
    child_builder.emit_abx(Op::LoadK, 1, u64::from(returned));
    child_builder.emit_abc(Op::Return, 1, 1, 0);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(2, 0, false);
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::Closure, 0, u64::from(child_index));
    parent.emit_abc(Op::Call, 0, 1, 1);
    parent.emit_abc(Op::Return, 0, 1, 0);

    let mut coroutine =
        PrimitiveCoroutine::new(parent.finish()).expect("coroutine should be created");

    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Yield(vec![Value::integer(3)])
    );
    assert_eq!(coroutine.status(), ThreadStatus::Suspended);
    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Return(vec![Value::integer(5)])
    );
}

#[test]
fn coroutine_resume_values_flow_into_called_lua_frame() {
    let mut child_builder = ProtoBuilder::new().with_signature(1, 0, false);
    let yielded = child_builder.add_constant(Value::integer(3));
    child_builder.emit_abx(Op::LoadK, 0, u64::from(yielded));
    child_builder.emit_abc(Op::Yield, 0, 1, 0);
    child_builder.emit_abc(Op::Return, 0, 1, 0);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(2, 0, false);
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::Closure, 0, u64::from(child_index));
    parent.emit_abc(Op::Call, 0, 1, 1);
    parent.emit_abc(Op::Return, 0, 1, 0);

    let mut coroutine =
        PrimitiveCoroutine::new(parent.finish()).expect("coroutine should be created");

    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Yield(vec![Value::integer(3)])
    );
    assert_eq!(
        coroutine.resume(&[Value::integer(8)]),
        CoroutineResume::Return(vec![Value::integer(8)])
    );
}

#[test]
fn coroutine_reports_dead_resume_error() {
    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let returned = builder.add_constant(Value::integer(1));
    builder.emit_abx(Op::LoadK, 0, u64::from(returned));
    builder.emit_abc(Op::Return, 0, 1, 0);

    let mut coroutine =
        PrimitiveCoroutine::new(builder.finish()).expect("coroutine should be created");

    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Return(vec![Value::integer(1)])
    );
    match coroutine.resume(&[]) {
        CoroutineResume::Error(error) => assert_eq!(error.kind(), &RuntimeErrorKind::CoroutineDead),
        result => panic!("expected dead coroutine error, got {result:?}"),
    }
}

#[test]
fn coroutine_errors_finish_thread() {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    builder.emit_abc(Op::LoadBool, 0, 1, 0);
    builder.emit_abc(Op::LoadBool, 1, 0, 0);
    builder.emit_abc(Op::Add, 2, 0, 1);

    let mut coroutine =
        PrimitiveCoroutine::new(builder.finish()).expect("coroutine should be created");

    match coroutine.resume(&[]) {
        CoroutineResume::Error(error) => {
            assert_eq!(
                error.kind(),
                &RuntimeErrorKind::NonNumericOperand { op: Op::Add }
            );
        }
        result => panic!("expected coroutine error, got {result:?}"),
    }
    assert_eq!(coroutine.status(), ThreadStatus::Dead);
}

#[test]
fn yield_outside_coroutine_is_runtime_error() {
    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let yielded = builder.add_constant(Value::integer(1));
    builder.emit_abx(Op::LoadK, 0, u64::from(yielded));
    builder.emit_abc(Op::Yield, 0, 1, 0);

    assert_runtime_error_kind(
        execute_proto(&builder.finish()),
        RuntimeErrorKind::YieldOutsideCoroutine,
    );
}

#[test]
fn to_be_closed_close_calls_close_metamethod_in_reverse_order() {
    let mut first_close = ProtoBuilder::new().with_signature(1, 0, false);
    let first_closed_name = first_close.add_string_constant("closed");
    let one = first_close.add_constant(Value::integer(1));
    first_close.emit_abx(Op::LoadK, 0, u64::from(one));
    first_close.emit_abx(Op::SetEnv, 0, u64::from(first_closed_name));
    first_close.emit_abc(Op::Return, 0, 0, 0);
    let first_close = first_close.finish();

    let mut second_close = ProtoBuilder::new().with_signature(1, 0, false);
    let second_closed_name = second_close.add_string_constant("closed");
    let two = second_close.add_constant(Value::integer(2));
    second_close.emit_abx(Op::LoadK, 0, u64::from(two));
    second_close.emit_abx(Op::SetEnv, 0, u64::from(second_closed_name));
    second_close.emit_abc(Op::Return, 0, 0, 0);
    let second_close = second_close.finish();

    let mut closures = vec![
        super::RuntimeClosure {
            proto: first_close,
            upvalues: Vec::new(),
        },
        super::RuntimeClosure {
            proto: second_close,
            upvalues: Vec::new(),
        },
    ];
    let mut tables = RuntimeTables::new();
    let first_table = tables.push_table(Table::new());
    let second_table = tables.push_table(Table::new());
    let mut strings = RuntimeStrings::new();
    let close_key = strings.intern_short_value("__close");
    let mut first_metatable = Table::new();
    assert!(first_metatable.raw_set_value(close_key, Value::closure_index(0)));
    let first_metatable = tables.push_table(first_metatable);
    let mut second_metatable = Table::new();
    assert!(second_metatable.raw_set_value(close_key, Value::closure_index(1)));
    let second_metatable = tables.push_table(second_metatable);
    tables
        .set_metatable(first_table as usize, Some(first_metatable))
        .expect("metatable link should be valid");
    tables
        .set_metatable(second_table as usize, Some(second_metatable))
        .expect("metatable link should be valid");
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut to_be_closed = Vec::new();
    let mut thread = elara_core::LuaThread::new();
    thread.push_value(Value::table_index(first_table));
    thread.push_value(Value::table_index(second_table));

    {
        let mut context = ExecutionContext {
            closures: &mut closures,
            tables: &mut tables,
            strings: &mut strings,
            natives: &natives,
            globals: &mut globals,
            to_be_closed: &mut to_be_closed,
        };
        execute_tbc(&thread, &mut context, Instr::abc(Op::Tbc, 0, 0, 0))
            .expect("first table should be closable");
        execute_tbc(&thread, &mut context, Instr::abc(Op::Tbc, 1, 0, 0))
            .expect("second table should be closable");
    }
    close_to_base(
        &thread,
        &mut closures,
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
        &mut to_be_closed,
        0,
    )
    .expect("close should execute");

    let closed_key = strings.intern_short_value("closed");
    let global_table = globals.value().as_table_index().expect("global table");
    assert_eq!(
        tables[global_table as usize].raw_get_value(closed_key),
        Value::integer(1)
    );
}

#[test]
fn to_be_closed_tbc_rejects_non_closable_values() {
    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    builder.emit_abc(Op::LoadInt, 0, 1, 0);
    builder.emit_abc(Op::Tbc, 0, 0, 0);

    assert_runtime_error_kind(
        execute_proto(&builder.finish()),
        RuntimeErrorKind::NonClosableValue,
    );
}

#[test]
fn to_be_closed_nil_and_false_do_not_require_close() {
    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let value = builder.add_constant(Value::integer(7));
    builder.emit_abc(Op::LoadNil, 0, 0, 0);
    builder.emit_abc(Op::LoadBool, 1, 0, 0);
    builder.emit_abc(Op::Tbc, 0, 0, 0);
    builder.emit_abc(Op::Tbc, 1, 0, 0);
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.emit_abc(Op::Return, 0, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(7)])
    );
}

#[test]
fn to_be_closed_error_unwind_runs_close_metamethod() {
    let mut close_builder = ProtoBuilder::new().with_signature(2, 0, false);
    let closed_name = close_builder.add_string_constant("closed");
    let one = close_builder.add_constant(Value::integer(1));
    close_builder.emit_abx(Op::LoadK, 0, u64::from(one));
    close_builder.emit_abx(Op::SetEnv, 0, u64::from(closed_name));
    close_builder.emit_abc(Op::Return, 0, 0, 0);

    let mut closures = vec![super::RuntimeClosure {
        proto: close_builder.finish(),
        upvalues: Vec::new(),
    }];
    let mut tables = RuntimeTables::new();
    let table = tables.push_table(Table::new());
    let mut strings = RuntimeStrings::new();
    let close_key = strings.intern_short_value("__close");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(close_key, Value::closure_index(0)));
    let metatable = tables.push_table(metatable);
    tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    let mut globals = runtime_globals(&mut tables);
    let natives = RuntimeNatives::new();
    let mut to_be_closed = Vec::new();
    let mut thread = elara_core::LuaThread::new();
    thread.push_value(Value::table_index(table));
    thread.push_value(Value::boolean(true));
    thread.push_value(Value::boolean(false));
    thread.push_value(Value::nil());

    {
        let mut context = ExecutionContext {
            closures: &mut closures,
            tables: &mut tables,
            strings: &mut strings,
            natives: &natives,
            globals: &mut globals,
            to_be_closed: &mut to_be_closed,
        };
        execute_tbc(&thread, &mut context, Instr::abc(Op::Tbc, 0, 0, 0))
            .expect("table should be closable");
    }
    let error = super::execute_arithmetic(
        &mut thread,
        &mut closures,
        Instr::abc(Op::Add, 3, 1, 2),
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
    )
    .unwrap_err();
    close_to_base(
        &thread,
        &mut closures,
        &mut tables,
        &mut strings,
        &natives,
        &mut globals,
        &mut to_be_closed,
        0,
    )
    .expect("error close should execute");

    assert_eq!(
        error.kind(),
        &RuntimeErrorKind::NonNumericOperand { op: Op::Add }
    );
    let closed_key = strings.intern_short_value("closed");
    let global_table = globals.value().as_table_index().expect("global table");
    assert_eq!(
        tables[global_table as usize].raw_get_value(closed_key),
        Value::integer(1)
    );
}

#[test]
fn to_be_closed_coroutine_yield_does_not_close_until_finish() {
    let mut close_builder = ProtoBuilder::new().with_signature(1, 0, false);
    let closed_name = close_builder.add_string_constant("closed");
    let one = close_builder.add_constant(Value::integer(1));
    close_builder.emit_abx(Op::LoadK, 0, u64::from(one));
    close_builder.emit_abx(Op::SetEnv, 0, u64::from(closed_name));
    close_builder.emit_abc(Op::Return, 0, 0, 0);

    let mut coroutine =
        PrimitiveCoroutine::new(ProtoBuilder::new().with_signature(0, 0, false).finish())
            .expect("empty coroutine should be created");
    coroutine.closures.push(super::RuntimeClosure {
        proto: close_builder.finish(),
        upvalues: Vec::new(),
    });
    let table = coroutine.tables.push_table(Table::new());
    let close_key = coroutine.strings.intern_short_value("__close");
    let mut metatable = Table::new();
    assert!(metatable.raw_set_value(close_key, Value::closure_index(0)));
    let metatable = coroutine.tables.push_table(metatable);
    coroutine
        .tables
        .set_metatable(table as usize, Some(metatable))
        .expect("metatable link should be valid");
    coroutine.thread.push_value(Value::table_index(table));
    coroutine.thread.push_value(Value::integer(7));

    let mut frame = CoroutineFrame::test_root(
        {
            let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
            builder.emit_abc(Op::Tbc, 0, 0, 0);
            builder.emit_abc(Op::Yield, 1, 1, 0);
            builder.emit_abc(Op::Return, 1, 1, 0);
            builder.finish()
        },
        0,
    );
    frame.pc = 0;
    coroutine.frames = vec![frame];

    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Yield(vec![Value::integer(7)])
    );
    let closed_key = coroutine.strings.intern_short_value("closed");
    let global_table = coroutine
        .globals
        .value()
        .as_table_index()
        .expect("global table");
    assert_eq!(
        coroutine.tables[global_table as usize].raw_get_value(closed_key),
        Value::nil()
    );
    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Return(vec![Value::integer(7)])
    );
    assert_eq!(
        coroutine.tables[global_table as usize].raw_get_value(closed_key),
        Value::integer(1)
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

    assert_runtime_error_kind(
        execute_proto(&builder.finish()),
        RuntimeErrorKind::NonCallableValue,
    );
}

#[test]
fn calls_attach_child_traceback_frame_to_runtime_errors() {
    let mut child_builder = ProtoBuilder::new()
        .with_signature(3, 0, false)
        .with_source_name("child.lua");
    child_builder.emit_abc(Op::LoadBool, 0, 1, 0);
    child_builder.emit_abc(Op::LoadBool, 1, 0, 0);
    child_builder.emit_abc(Op::Add, 2, 0, 1);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(1, 0, false);
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::Closure, 0, u64::from(child_index));
    parent.emit_abc(Op::Call, 0, 1, 1);

    let error = execute_proto(&parent.finish()).expect_err("expected runtime error");

    assert_eq!(
        error.kind(),
        &RuntimeErrorKind::NonNumericOperand { op: Op::Add }
    );
    assert_eq!(error.traceback().len(), 1);
    assert_eq!(error.traceback()[0].source(), Some("child.lua"));
    assert_eq!(error.traceback()[0].function(), Some("child.lua"));
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
fn conditionals_execute_test_and_jump() {
    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let then_value = builder.add_constant(Value::integer(1));
    let else_value = builder.add_constant(Value::integer(2));
    builder.emit_abc(Op::LoadBool, 0, 0, 0);
    builder.emit_abc(Op::Test, 0, 0, 0);
    builder.emit_asbx(Op::Jmp, 0, 2);
    builder.emit_abx(Op::LoadK, 1, u64::from(then_value));
    builder.emit_abc(Op::Return, 1, 1, 0);
    builder.emit_abx(Op::LoadK, 1, u64::from(else_value));
    builder.emit_abc(Op::Return, 1, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(2)])
    );
}

#[test]
fn loops_execute_backward_jump_until_break() {
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let zero = builder.add_constant(Value::integer(0));
    let one = builder.add_constant(Value::integer(1));
    builder.emit_abx(Op::LoadK, 0, u64::from(zero));
    builder.emit_abc(Op::LoadBool, 1, 1, 0);
    builder.emit_abc(Op::Test, 1, 0, 0);
    builder.emit_asbx(Op::Jmp, 0, 4);
    builder.emit_abx(Op::LoadK, 2, u64::from(one));
    builder.emit_abc(Op::Add, 0, 0, 2);
    builder.emit_asbx(Op::Jmp, 0, 1);
    builder.emit_asbx(Op::Jmp, 0, -7);
    builder.emit_abc(Op::Return, 0, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(1)])
    );
}

#[test]
fn numeric_for_executes_integer_positive_step() {
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let zero = builder.add_constant(Value::integer(0));
    let one = builder.add_constant(Value::integer(1));
    let three = builder.add_constant(Value::integer(3));
    builder.emit_abx(Op::LoadK, 3, u64::from(zero));
    builder.emit_abx(Op::LoadK, 0, u64::from(one));
    builder.emit_abx(Op::LoadK, 1, u64::from(three));
    builder.emit_abx(Op::LoadK, 2, u64::from(one));
    builder.emit_asbx(Op::ForPrep, 0, 2);
    builder.emit_abc(Op::Add, 3, 3, 2);
    builder.emit_asbx(Op::ForLoop, 0, -2);
    builder.emit_abc(Op::Return, 3, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(6)])
    );
}

#[test]
fn numeric_for_executes_integer_negative_step() {
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let zero = builder.add_constant(Value::integer(0));
    let one = builder.add_constant(Value::integer(1));
    let three = builder.add_constant(Value::integer(3));
    let negative_one = builder.add_constant(Value::integer(-1));
    builder.emit_abx(Op::LoadK, 3, u64::from(zero));
    builder.emit_abx(Op::LoadK, 0, u64::from(three));
    builder.emit_abx(Op::LoadK, 1, u64::from(one));
    builder.emit_abx(Op::LoadK, 2, u64::from(negative_one));
    builder.emit_asbx(Op::ForPrep, 0, 2);
    builder.emit_abc(Op::Add, 3, 3, 2);
    builder.emit_asbx(Op::ForLoop, 0, -2);
    builder.emit_abc(Op::Return, 3, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(6)])
    );
}

#[test]
fn numeric_for_executes_float_step() {
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let zero = builder.add_constant(Value::float(0.0));
    let init = builder.add_constant(Value::float(1.5));
    let limit = builder.add_constant(Value::float(2.5));
    let step = builder.add_constant(Value::float(0.5));
    builder.emit_abx(Op::LoadK, 3, u64::from(zero));
    builder.emit_abx(Op::LoadK, 0, u64::from(init));
    builder.emit_abx(Op::LoadK, 1, u64::from(limit));
    builder.emit_abx(Op::LoadK, 2, u64::from(step));
    builder.emit_asbx(Op::ForPrep, 0, 2);
    builder.emit_abc(Op::Add, 3, 3, 2);
    builder.emit_asbx(Op::ForLoop, 0, -2);
    builder.emit_abc(Op::Return, 3, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::float(6.0)])
    );
}

#[test]
fn numeric_for_rejects_zero_step() {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let one = builder.add_constant(Value::integer(1));
    let zero = builder.add_constant(Value::integer(0));
    builder.emit_abx(Op::LoadK, 0, u64::from(one));
    builder.emit_abx(Op::LoadK, 1, u64::from(one));
    builder.emit_abx(Op::LoadK, 2, u64::from(zero));
    builder.emit_asbx(Op::ForPrep, 0, 0);

    assert_runtime_error_kind(
        execute_proto(&builder.finish()),
        RuntimeErrorKind::ForLoopStepZero,
    );
}

#[test]
fn generic_for_executes_iterator_result() {
    let mut child_builder = ProtoBuilder::new().with_signature(1, 0, false);
    let value = child_builder.add_constant(Value::integer(7));
    child_builder.emit_abx(Op::LoadK, 0, u64::from(value));
    child_builder.emit_abc(Op::Return, 0, 1, 0);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(5, 0, false);
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::Closure, 0, u64::from(child_index));
    parent.emit_abc(Op::LoadNil, 1, 0, 0);
    parent.emit_abc(Op::LoadNil, 2, 0, 0);
    parent.emit_asbx(Op::TForPrep, 0, 1);
    parent.emit_abc(Op::Return, 3, 1, 0);
    parent.emit_abc(Op::TForCall, 0, 0, 1);
    parent.emit_asbx(Op::TForLoop, 0, -3);
    let fallback = parent.add_constant(Value::integer(0));
    parent.emit_abx(Op::LoadK, 4, u64::from(fallback));
    parent.emit_abc(Op::Return, 4, 1, 0);

    assert_eq!(execute_proto(&parent.finish()), Ok(vec![Value::integer(7)]));
}

#[test]
fn generic_for_skips_body_on_nil_iterator_result() {
    let mut child_builder = ProtoBuilder::new().with_signature(1, 0, false);
    child_builder.emit_abc(Op::LoadNil, 0, 0, 0);
    child_builder.emit_abc(Op::Return, 0, 1, 0);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(5, 0, false);
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::Closure, 0, u64::from(child_index));
    parent.emit_abc(Op::LoadNil, 1, 0, 0);
    parent.emit_abc(Op::LoadNil, 2, 0, 0);
    parent.emit_asbx(Op::TForPrep, 0, 2);
    let body_value = parent.add_constant(Value::integer(99));
    parent.emit_abx(Op::LoadK, 3, u64::from(body_value));
    parent.emit_abc(Op::Return, 3, 1, 0);
    parent.emit_abc(Op::TForCall, 0, 0, 1);
    parent.emit_asbx(Op::TForLoop, 0, -4);
    let fallback = parent.add_constant(Value::integer(42));
    parent.emit_abx(Op::LoadK, 4, u64::from(fallback));
    parent.emit_abc(Op::Return, 4, 1, 0);

    assert_eq!(
        execute_proto(&parent.finish()),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn table_constructor_executes_array_record_and_keyed_fields() {
    let mut builder = ProtoBuilder::new().with_signature(7, 0, false);
    let one = builder.add_constant(Value::integer(1));
    let two = builder.add_constant(Value::integer(2));
    let three = builder.add_constant(Value::integer(3));
    let four = builder.add_constant(Value::integer(4));
    let name = builder.add_string_constant("named");
    builder.emit_abc(Op::NewTable, 0, 1, 2);
    builder.emit_abx(Op::LoadK, 1, u64::from(one));
    builder.emit_abx(Op::LoadK, 2, u64::from(two));
    builder.emit_abc(Op::SetTable, 0, 1, 2);
    builder.emit_abx(Op::LoadString, 3, u64::from(name));
    builder.emit_abx(Op::LoadK, 4, u64::from(three));
    builder.emit_abc(Op::SetTable, 0, 3, 4);
    builder.emit_abc(Op::LoadBool, 5, 1, 0);
    builder.emit_abx(Op::LoadK, 6, u64::from(four));
    builder.emit_abc(Op::SetTable, 0, 5, 6);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let mut output = execute_proto_with_output(&builder.finish()).expect("execution should pass");
    let table_index = output.values[0]
        .as_table_index()
        .expect("expected table placeholder");
    let name_key = output.strings.intern_short_value("named");
    let table = &output.tables[table_index as usize];

    assert_eq!(table.raw_get_integer(1), Value::integer(2));
    assert_eq!(table.raw_get_value(name_key), Value::integer(3));
    assert_eq!(table.raw_get_value(Value::boolean(true)), Value::integer(4));
}

#[test]
fn table_constructor_rejects_nil_key() {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let value = builder.add_constant(Value::integer(1));
    builder.emit_abc(Op::NewTable, 0, 0, 1);
    builder.emit_abc(Op::LoadNil, 1, 0, 0);
    builder.emit_abx(Op::LoadK, 2, u64::from(value));
    builder.emit_abc(Op::SetTable, 0, 1, 2);

    assert_runtime_error_kind(
        execute_proto(&builder.finish()),
        RuntimeErrorKind::InvalidTableKey,
    );
}

#[test]
fn table_access_executes_generic_hash_get_and_set() {
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let key = builder.add_string_constant("answer");
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abc(Op::NewTable, 0, 0, 1);
    builder.emit_abx(Op::LoadString, 1, u64::from(key));
    builder.emit_abx(Op::LoadK, 2, u64::from(value));
    builder.emit_abc(Op::SetTable, 0, 1, 2);
    builder.emit_abc(Op::GetTable, 3, 0, 1);
    builder.emit_abc(Op::Return, 3, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn table_access_executes_integer_index_fast_path() {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abc(Op::NewTable, 0, 1, 0);
    builder.emit_abx(Op::LoadK, 1, u64::from(value));
    builder.emit_abc(Op::SetIndex, 0, 1, 1);
    builder.emit_abc(Op::GetIndex, 2, 0, 1);
    builder.emit_abc(Op::Return, 2, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn table_access_nil_assignment_clears_integer_slot() {
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abc(Op::NewTable, 0, 1, 0);
    builder.emit_abx(Op::LoadK, 1, u64::from(value));
    builder.emit_abc(Op::SetIndex, 0, 1, 1);
    builder.emit_abc(Op::LoadNil, 2, 0, 0);
    builder.emit_abc(Op::SetIndex, 0, 1, 2);
    builder.emit_abc(Op::GetIndex, 3, 0, 1);
    builder.emit_abc(Op::Return, 3, 1, 0);

    assert_eq!(execute_proto(&builder.finish()), Ok(vec![Value::nil()]));
}

#[test]
fn globals_execute_set_get_env() {
    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let name = builder.add_string_constant("answer");
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.emit_abx(Op::SetEnv, 0, u64::from(name));
    builder.emit_abx(Op::GetEnv, 1, u64::from(name));
    builder.emit_abc(Op::Return, 1, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn globals_default_env_upvalue_matches_get_set_env_table() {
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    builder.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("_ENV"), false, 0));
    let name = builder.add_string_constant("answer");
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.emit_abx(Op::SetEnv, 0, u64::from(name));
    builder.emit_abc(Op::GetUpvalue, 1, 0, 0);
    builder.emit_abx(Op::LoadString, 2, u64::from(name));
    builder.emit_abc(Op::GetTable, 3, 1, 2);
    builder.emit_abc(Op::Return, 3, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn globals_declaration_rejects_existing_value() {
    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let name = builder.add_string_constant("answer");
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.emit_abx(Op::SetEnv, 0, u64::from(name));
    builder.emit_abx(Op::GetEnv, 1, u64::from(name));
    builder.emit_abx(Op::DeclGlobal, 1, u64::from(name));
    builder.emit_abc(Op::Return, 1, 1, 0);

    assert_runtime_error_kind(
        execute_proto(&builder.finish()),
        RuntimeErrorKind::GlobalAlreadyDefined,
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
