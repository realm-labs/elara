use elara_bytecode::{Instr, Op, ProtoBuilder};
use elara_core::{Table, ThreadStatus, Value};

use super::{
    CoroutineFrame, CoroutineResume, ExecutionContext, PrimitiveCoroutine, ProtectedRuntimeOutput,
    RuntimeDebugHooks, RuntimeEnvironment, RuntimeErrorKind, RuntimeGlobals, RuntimeNatives,
    RuntimeStrings, RuntimeTables, close_to_base, execute_proto, execute_proto_protected,
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
fn superinstruction_add_int_executes_integer_immediate_addition() {
    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let left = builder.add_constant(Value::integer(5));
    builder.emit_abx(Op::LoadK, 0, u64::from(left));
    builder.emit_abc(Op::AddInt, 1, 0, 7);
    builder.emit_abc(Op::Return, 1, 1, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![Value::integer(12)])
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
fn arithmetic_executes_integer_bitwise_operations() {
    let mut builder = ProtoBuilder::new().with_signature(7, 0, false);
    let left = builder.add_constant(Value::integer(0b1010));
    let right = builder.add_constant(Value::integer(0b1100));
    builder.emit_abx(Op::LoadK, 0, u64::from(left));
    builder.emit_abx(Op::LoadK, 1, u64::from(right));
    builder.emit_abc(Op::BAnd, 2, 0, 1);
    builder.emit_abc(Op::BOr, 3, 0, 1);
    builder.emit_abc(Op::BXor, 4, 0, 1);
    builder.emit_abc(Op::BNot, 5, 0, 0);
    builder.emit_abc(Op::Return, 2, 4, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![
            Value::integer(0b1000),
            Value::integer(0b1110),
            Value::integer(0b0110),
            Value::integer(!0b1010),
        ])
    );
}

#[test]
fn arithmetic_executes_integer_shift_operations() {
    let mut builder = ProtoBuilder::new().with_signature(8, 0, false);
    let value = builder.add_constant(Value::integer(8));
    let two = builder.add_constant(Value::integer(2));
    let wide = builder.add_constant(Value::integer(64));
    let negative = builder.add_constant(Value::integer(-1));
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.emit_abx(Op::LoadK, 1, u64::from(two));
    builder.emit_abx(Op::LoadK, 2, u64::from(wide));
    builder.emit_abx(Op::LoadK, 3, u64::from(negative));
    builder.emit_abc(Op::Shl, 4, 0, 1);
    builder.emit_abc(Op::Shr, 5, 0, 1);
    builder.emit_abc(Op::Shl, 6, 0, 2);
    builder.emit_abc(Op::Shr, 7, 0, 3);
    builder.emit_abc(Op::Return, 4, 4, 0);

    assert_eq!(
        execute_proto(&builder.finish()),
        Ok(vec![
            Value::integer(32),
            Value::integer(2),
            Value::integer(0),
            Value::integer(16),
        ])
    );
}

#[test]
fn native_functions_execute_call() {
    let natives = RuntimeNatives::new();
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
fn native_context_materializes_debug_info_for_lua_caller() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("capture", |context, _args| {
        Ok(vec![context.debug_info_for_level(1, b"Slf")?])
    });

    let mut builder = ProtoBuilder::new()
        .with_signature(1, 0, false)
        .with_source_name("debug.lua");
    let capture = builder.add_string_constant("capture");
    builder.emit_line(Instr::abx(Op::GetEnv, 0, u64::from(capture)), 10);
    builder.emit_line(Instr::abc(Op::Call, 0, 1, 1), 11);
    builder.emit_line(Instr::abc(Op::Return, 0, 1, 0), 12);

    let mut output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("debug info capture should execute");
    let info = output.values[0]
        .as_table_index()
        .expect("debug info should be a table") as usize;
    let source = output.strings.intern_short_value("source");
    let currentline = output.strings.intern_short_value("currentline");
    let what = output.strings.intern_short_value("what");
    let func = output.strings.intern_short_value("func");
    let table = output.tables.get(info).expect("debug info table");

    let source = table.raw_get_value(source);
    assert_eq!(
        output.strings.string_bytes(source),
        Some(b"debug.lua" as &[u8])
    );
    assert_eq!(table.raw_get_value(currentline), Value::integer(11));
    let what = table.raw_get_value(what);
    assert_eq!(output.strings.string_bytes(what), Some(b"main" as &[u8]));
    assert_eq!(table.raw_get_value(func), Value::nil());
}

#[test]
fn native_context_reads_lua_caller_locals() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("capture", |context, _args| {
        let Some((name, value)) = context.debug_getlocal(1, 1)? else {
            return Ok(vec![Value::boolean(false)]);
        };
        Ok(vec![Value::boolean(
            context.string_bytes(name) == Some(b"x" as &[u8]) && value == Value::integer(42),
        )])
    });

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let value = builder.add_constant(Value::integer(42));
    let capture = builder.add_string_constant("capture");
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.add_local_var("x", 0, 1, u32::MAX);
    builder.emit_abx(Op::GetEnv, 1, u64::from(capture));
    builder.emit_abc(Op::Call, 1, 1, 1);
    builder.emit_abc(Op::Return, 1, 1, 0);

    assert_eq!(
        execute_proto_with_environment(&builder.finish(), environment).map(|output| output.values),
        Ok(vec![Value::boolean(true)])
    );
}

#[test]
fn native_context_reads_function_target_parameter_names() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("capture", |context, args| {
        let Some(function) = args.first().copied() else {
            return Ok(vec![Value::boolean(false)]);
        };
        let Some(name) = context.debug_getlocal_function(function, 1)? else {
            return Ok(vec![Value::boolean(false)]);
        };
        Ok(vec![Value::boolean(
            context.string_bytes(name) == Some(b"arg" as &[u8]),
        )])
    });

    let mut child = ProtoBuilder::new().with_signature(1, 1, false);
    child.add_local_var("arg", 0, 0, u32::MAX);
    child.emit_abc(Op::Return, 0, 0, 0);

    let mut parent = ProtoBuilder::new().with_signature(3, 0, false);
    let capture = parent.add_string_constant("capture");
    let child_index = parent.add_child(child.finish());
    parent.emit_abx(Op::GetEnv, 0, u64::from(capture));
    parent.emit_abx(Op::Closure, 1, u64::from(child_index));
    parent.emit_abc(Op::Call, 0, 2, 1);
    parent.emit_abc(Op::Return, 0, 1, 0);

    assert_eq!(
        execute_proto_with_environment(&parent.finish(), environment).map(|output| output.values),
        Ok(vec![Value::boolean(true)])
    );
}

#[test]
fn native_context_sets_lua_caller_locals() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("mutate", |context, _args| {
        let Some(name) = context.debug_setlocal(1, 1, Value::integer(99))? else {
            return Ok(vec![Value::boolean(false)]);
        };
        Ok(vec![Value::boolean(
            context.string_bytes(name) == Some(b"x" as &[u8]),
        )])
    });

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let initial = builder.add_constant(Value::integer(42));
    let mutate = builder.add_string_constant("mutate");
    builder.emit_abx(Op::LoadK, 0, u64::from(initial));
    builder.add_local_var("x", 0, 1, u32::MAX);
    builder.emit_abx(Op::GetEnv, 1, u64::from(mutate));
    builder.emit_abc(Op::Call, 1, 1, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    assert_eq!(
        execute_proto_with_environment(&builder.finish(), environment).map(|output| output.values),
        Ok(vec![Value::integer(99)])
    );
}

#[test]
fn native_context_reads_lua_closure_upvalues() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("capture", |context, args| {
        let Some((name, value)) = args
            .first()
            .copied()
            .map(|function| context.debug_getupvalue(function, 1))
            .transpose()?
            .flatten()
        else {
            return Ok(vec![Value::boolean(false)]);
        };
        let is_expected =
            context.string_bytes(name) == Some(b"x" as &[u8]) && value == Value::integer(41);
        Ok(vec![Value::boolean(is_expected)])
    });

    let mut child_builder = ProtoBuilder::new().with_signature(1, 0, false);
    child_builder.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
    child_builder.emit_abc(Op::Return, 0, 0, 0);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(4, 0, false);
    let value = parent.add_constant(Value::integer(41));
    let capture = parent.add_string_constant("capture");
    parent.emit_abx(Op::LoadK, 0, u64::from(value));
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::Closure, 1, u64::from(child_index));
    parent.emit_abx(Op::GetEnv, 2, u64::from(capture));
    parent.emit_abc(Op::Move, 3, 1, 0);
    parent.emit_abc(Op::Call, 2, 2, 1);
    parent.emit_abc(Op::Return, 2, 1, 0);

    assert_eq!(
        execute_proto_with_environment(&parent.finish(), environment).map(|output| output.values),
        Ok(vec![Value::boolean(true)])
    );
}

#[test]
fn native_context_sets_lua_closure_upvalues() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("capture", |context, args| {
        let Some(name) = args
            .first()
            .copied()
            .map(|function| context.debug_setupvalue(function, 1, Value::integer(42)))
            .transpose()?
            .flatten()
        else {
            return Ok(vec![Value::boolean(false)]);
        };
        Ok(vec![Value::boolean(
            context.string_bytes(name) == Some(b"x" as &[u8]),
        )])
    });

    let mut child_builder = ProtoBuilder::new().with_signature(1, 0, false);
    child_builder.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
    child_builder.emit_abc(Op::GetUpvalue, 0, 0, 0);
    child_builder.emit_abc(Op::Return, 0, 1, 0);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(4, 0, false);
    let value = parent.add_constant(Value::integer(41));
    let capture = parent.add_string_constant("capture");
    parent.emit_abx(Op::LoadK, 0, u64::from(value));
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::Closure, 1, u64::from(child_index));
    parent.emit_abx(Op::GetEnv, 2, u64::from(capture));
    parent.emit_abc(Op::Move, 3, 1, 0);
    parent.emit_abc(Op::Call, 2, 2, 1);
    parent.emit_abc(Op::Call, 1, 1, 1);
    parent.emit_abc(Op::Return, 1, 2, 0);

    assert_eq!(
        execute_proto_with_environment(&parent.finish(), environment).map(|output| output.values),
        Ok(vec![Value::integer(42), Value::boolean(true)])
    );
}

#[test]
fn sibling_closures_share_parent_stack_upvalue_cells() {
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("mutate", |context, args| {
        let Some(name) = args
            .first()
            .copied()
            .map(|function| context.debug_setupvalue(function, 1, Value::integer(42)))
            .transpose()?
            .flatten()
        else {
            return Ok(Vec::new());
        };
        assert_eq!(context.string_bytes(name), Some(b"x" as &[u8]));
        Ok(Vec::new())
    });

    let mut first_child = ProtoBuilder::new().with_signature(1, 0, false);
    first_child.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
    first_child.emit_abc(Op::GetUpvalue, 0, 0, 0);
    first_child.emit_abc(Op::Return, 0, 1, 0);
    let first_child = first_child.finish();

    let mut second_child = ProtoBuilder::new().with_signature(1, 0, false);
    second_child.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
    second_child.emit_abc(Op::GetUpvalue, 0, 0, 0);
    second_child.emit_abc(Op::Return, 0, 1, 0);
    let second_child = second_child.finish();

    let mut parent = ProtoBuilder::new().with_signature(5, 0, false);
    let initial = parent.add_constant(Value::integer(41));
    let mutate = parent.add_string_constant("mutate");
    parent.emit_abx(Op::LoadK, 0, u64::from(initial));
    let first_index = parent.add_child(first_child);
    parent.emit_abx(Op::Closure, 1, u64::from(first_index));
    let second_index = parent.add_child(second_child);
    parent.emit_abx(Op::Closure, 2, u64::from(second_index));
    parent.emit_abx(Op::GetEnv, 3, u64::from(mutate));
    parent.emit_abc(Op::Move, 4, 1, 0);
    parent.emit_abc(Op::Call, 3, 2, 1);
    parent.emit_abc(Op::Call, 2, 1, 1);
    parent.emit_abc(Op::Return, 2, 1, 0);

    assert_eq!(
        execute_proto_with_environment(&parent.finish(), environment).map(|output| output.values),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn cloned_native_registries_share_later_registrations() {
    let natives = RuntimeNatives::new();
    let shared = natives.clone();
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

    let output = execute_proto_with_natives(&builder.finish(), shared)
        .expect("shared native registration should execute");
    assert_eq!(output.values, vec![Value::integer(42)]);
}

#[test]
fn native_context_can_create_callable_native_function() {
    let natives = RuntimeNatives::new();
    let factory = natives.push(|context, _args| {
        Ok(vec![context.create_native_function(|_context, _args| {
            Ok(vec![Value::integer(42)])
        })])
    });

    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let callee = builder.add_constant(Value::native_function_index(factory));
    builder.emit_abx(Op::LoadK, 0, u64::from(callee));
    builder.emit_abc(Op::Call, 0, 1, 1);
    builder.emit_abc(Op::Call, 0, 1, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let output = execute_proto_with_natives(&builder.finish(), natives)
        .expect("dynamically registered native should execute");
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
    let natives = RuntimeNatives::new();
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
fn initial_global_tables_can_seed_string_fields() {
    let mut environment = RuntimeEnvironment::new();
    environment.set_global_table_with_string_fields(
        "utf8",
        std::iter::empty::<(&str, Value)>(),
        [("charpattern", b"pattern".as_slice())],
    );

    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let module = builder.add_string_constant("utf8");
    let field = builder.add_string_constant("charpattern");
    builder.emit_abx(Op::GetEnv, 0, u64::from(module));
    builder.emit_abx(Op::LoadString, 1, u64::from(field));
    builder.emit_abc(Op::GetTable, 0, 0, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let mut output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("registered string table field should execute");
    let value = output.values.pop().expect("field should return a value");
    assert_eq!(
        output.strings.short_string_bytes(value),
        Some(b"pattern".as_slice())
    );
}

#[test]
fn initial_global_tables_can_seed_long_string_fields() {
    let long_path = "./?.lua;./?/init.lua;./modules/?.lua;./modules/?/init.lua";
    let mut environment = RuntimeEnvironment::new();
    environment.set_global_table_with_string_fields(
        "package",
        std::iter::empty::<(&str, Value)>(),
        [("path", long_path.as_bytes())],
    );

    let mut builder = ProtoBuilder::new().with_signature(2, 0, false);
    let module = builder.add_string_constant("package");
    let field = builder.add_string_constant("path");
    builder.emit_abx(Op::GetEnv, 0, u64::from(module));
    builder.emit_abx(Op::LoadString, 1, u64::from(field));
    builder.emit_abc(Op::GetTable, 0, 0, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let mut output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("registered long string table field should execute");
    let value = output.values.pop().expect("field should return a value");
    assert_eq!(output.strings.short_string_bytes(value), None);
    assert_eq!(
        output.strings.string_bytes(value),
        Some(long_path.as_bytes())
    );
}

#[test]
fn initial_global_tables_can_seed_empty_table_fields() {
    let mut environment = RuntimeEnvironment::new();
    environment.set_global_table_with_string_and_empty_table_fields(
        "package",
        std::iter::empty::<(&str, Value)>(),
        std::iter::empty::<(&str, &[u8])>(),
        ["loaded", "preload"],
    );

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let module = builder.add_string_constant("package");
    let loaded = builder.add_string_constant("loaded");
    let preload = builder.add_string_constant("preload");
    builder.emit_abx(Op::GetEnv, 0, u64::from(module));
    builder.emit_abx(Op::LoadString, 1, u64::from(loaded));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abx(Op::LoadString, 2, u64::from(preload));
    builder.emit_abc(Op::GetTable, 2, 0, 2);
    builder.emit_abc(Op::Return, 1, 2, 0);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("registered empty table fields should execute");
    assert_eq!(output.values.len(), 2);
    assert!(output.values[0].is_table());
    assert!(output.values[1].is_table());
    assert_ne!(output.values[0], output.values[1]);
}

#[test]
fn initial_global_tables_can_seed_nested_value_table_fields() {
    let native = Value::native_function_index(3);
    let mut environment = RuntimeEnvironment::new();
    environment.set_global_table_with_string_and_table_fields(
        "package",
        std::iter::empty::<(&str, Value)>(),
        std::iter::empty::<(&str, &[u8])>(),
        [("searchers", vec![(Value::integer(1), native)])],
    );

    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let module = builder.add_string_constant("package");
    let searchers = builder.add_string_constant("searchers");
    builder.emit_abx(Op::GetEnv, 0, u64::from(module));
    builder.emit_abx(Op::LoadString, 1, u64::from(searchers));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abx(Op::LoadInt, 2, 1);
    builder.emit_abc(Op::GetTable, 1, 1, 2);
    builder.emit_abc(Op::Return, 1, 1, 0);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("registered nested table field should execute");
    assert_eq!(output.values, vec![native]);
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
fn native_functions_can_allocate_runtime_long_strings() {
    const LONG_LABEL: &str = "native string payload longer than short string storage";
    let mut environment = RuntimeEnvironment::new();
    environment.register_native_global("label", |context, _args| {
        Ok(vec![context.intern_string(LONG_LABEL)])
    });

    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let name = builder.add_string_constant("label");
    builder.emit_abx(Op::GetEnv, 0, u64::from(name));
    builder.emit_abc(Op::Call, 0, 1, 1);
    builder.emit_abc(Op::Return, 0, 1, 0);

    let mut output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("native long string allocation should execute");
    let value = output.values.pop().expect("native should return a value");
    assert_eq!(output.strings.short_string_bytes(value), None);
    assert_eq!(
        output.strings.string_bytes(value),
        Some(LONG_LABEL.as_bytes())
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
fn coroutine_materializes_debug_frames_for_native_calls() {
    let mut child = ProtoBuilder::new().with_signature(3, 0, false);
    let value = child.add_constant(Value::integer(42));
    let capture = child.add_string_constant("capture");
    child.emit_abx(Op::LoadK, 0, u64::from(value));
    child.add_local_var("x", 0, 1, u32::MAX);
    child.emit_abx(Op::GetEnv, 1, u64::from(capture));
    child.emit_abc(Op::Call, 1, 1, 1);
    child.emit_abc(Op::Return, 1, 1, 0);

    let mut parent = ProtoBuilder::new().with_signature(2, 0, false);
    let child_index = parent.add_child(child.finish());
    parent.emit_abx(Op::Closure, 0, u64::from(child_index));
    parent.emit_abc(Op::Call, 0, 1, 1);
    parent.emit_abc(Op::Return, 0, 1, 0);

    let mut coroutine =
        PrimitiveCoroutine::new(parent.finish()).expect("coroutine should be created");
    let native = coroutine.natives.push(|context, _args| {
        let Some((name, value)) = context.debug_getlocal(1, 1)? else {
            return Ok(vec![Value::boolean(false)]);
        };
        let what_key = context.intern_string("what");
        let child_info = context.debug_info_for_level(1, b"S")?;
        let parent_info = context.debug_info_for_level(2, b"S")?;
        let child_what = context.table_get(child_info, what_key)?;
        let parent_what = context.table_get(parent_info, what_key)?;
        Ok(vec![Value::boolean(
            context.string_bytes(name) == Some(b"x" as &[u8])
                && value == Value::integer(42)
                && context.string_bytes(child_what) == Some(b"Lua" as &[u8])
                && context.string_bytes(parent_what) == Some(b"main" as &[u8]),
        )])
    });
    let capture = coroutine.strings.intern_short_value("capture");
    let global_table = coroutine
        .globals
        .value()
        .as_table_index()
        .expect("global table");
    assert!(
        coroutine
            .tables
            .get_mut(global_table as usize)
            .expect("global table should exist")
            .raw_set_value(capture, Value::native_function_index(native))
    );

    assert_eq!(
        coroutine.resume(&[]),
        CoroutineResume::Return(vec![Value::boolean(true)])
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
        let mut debug_frames = Vec::new();
        let debug_hooks = RuntimeDebugHooks::new();
        let mut context = ExecutionContext {
            closures: &mut closures,
            tables: &mut tables,
            strings: &mut strings,
            natives: &natives,
            globals: &mut globals,
            to_be_closed: &mut to_be_closed,
            debug_frames: &mut debug_frames,
            debug_hooks: &debug_hooks,
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
        let mut debug_frames = Vec::new();
        let debug_hooks = RuntimeDebugHooks::new();
        let mut context = ExecutionContext {
            closures: &mut closures,
            tables: &mut tables,
            strings: &mut strings,
            natives: &natives,
            globals: &mut globals,
            to_be_closed: &mut to_be_closed,
            debug_frames: &mut debug_frames,
            debug_hooks: &debug_hooks,
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
fn closures_assign_shared_upvalue_cells() {
    let mut writer_builder = ProtoBuilder::new().with_signature(2, 0, false);
    writer_builder.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
    let one = writer_builder.add_constant(Value::integer(1));
    writer_builder.emit_abc(Op::GetUpvalue, 0, 0, 0);
    writer_builder.emit_abx(Op::LoadK, 1, u64::from(one));
    writer_builder.emit_abc(Op::Add, 0, 0, 1);
    writer_builder.emit_abc(Op::SetUpvalue, 0, 0, 0);
    writer_builder.emit_abc(Op::Return, 0, 1, 0);
    let writer = writer_builder.finish();

    let mut reader_builder = ProtoBuilder::new().with_signature(1, 0, false);
    reader_builder.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
    reader_builder.emit_abc(Op::GetUpvalue, 0, 0, 0);
    reader_builder.emit_abc(Op::Return, 0, 1, 0);
    let reader = reader_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(3, 0, false);
    let value = parent.add_constant(Value::integer(40));
    let writer_index = parent.add_child(writer);
    let reader_index = parent.add_child(reader);
    parent.emit_abx(Op::LoadK, 0, u64::from(value));
    parent.emit_abx(Op::Closure, 1, u64::from(writer_index));
    parent.emit_abx(Op::Closure, 2, u64::from(reader_index));
    parent.emit_abc(Op::Call, 1, 1, 1);
    parent.emit_abc(Op::Call, 2, 1, 1);
    parent.emit_abc(Op::Return, 1, 2, 0);

    assert_eq!(
        execute_proto(&parent.finish()),
        Ok(vec![Value::integer(41), Value::integer(41)])
    );
}

#[test]
fn closures_sync_open_upvalue_writes_to_parent_stack() {
    let mut child_builder = ProtoBuilder::new().with_signature(2, 0, false);
    child_builder.add_upvalue(elara_bytecode::UpvalueDesc::new(Some("x"), true, 0));
    let one = child_builder.add_constant(Value::integer(1));
    child_builder.emit_abc(Op::GetUpvalue, 0, 0, 0);
    child_builder.emit_abx(Op::LoadK, 1, u64::from(one));
    child_builder.emit_abc(Op::Add, 0, 0, 1);
    child_builder.emit_abc(Op::SetUpvalue, 0, 0, 0);
    child_builder.emit_abc(Op::Return, 0, 1, 0);
    let child = child_builder.finish();

    let mut parent = ProtoBuilder::new().with_signature(2, 0, false);
    let value = parent.add_constant(Value::integer(40));
    let child_index = parent.add_child(child);
    parent.emit_abx(Op::LoadK, 0, u64::from(value));
    parent.emit_abx(Op::Closure, 1, u64::from(child_index));
    parent.emit_abc(Op::Call, 1, 1, 1);
    parent.emit_abc(Op::Return, 0, 1, 0);

    assert_eq!(
        execute_proto(&parent.finish()),
        Ok(vec![Value::integer(41)])
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

    let mut output = execute_proto_with_output(&parent.finish()).expect("execution should pass");
    let table_index = output.values[0]
        .as_table_index()
        .expect("expected table placeholder");
    let n_key = output.strings.intern_short_value("n");
    let table = &output.tables[table_index as usize];

    assert_eq!(table.raw_get_integer(1), Value::integer(42));
    assert_eq!(table.raw_get_integer(2), Value::integer(99));
    assert_eq!(table.raw_get_integer(3), Value::nil());
    assert_eq!(table.raw_get_value(n_key), Value::integer(2));
}
