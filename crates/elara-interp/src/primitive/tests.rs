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

    assert_eq!(
        execute_proto(&builder.finish()),
        Err(RuntimeError::ForLoopStepZero)
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

    assert_eq!(
        execute_proto(&builder.finish()),
        Err(RuntimeError::InvalidTableKey)
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

    assert_eq!(
        execute_proto(&builder.finish()),
        Err(RuntimeError::GlobalAlreadyDefined)
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
