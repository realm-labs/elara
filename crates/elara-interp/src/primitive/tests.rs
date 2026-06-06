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
