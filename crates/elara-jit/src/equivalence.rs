//! JIT/interpreter equivalence tests.

use elara_bytecode::{Op, Proto, ProtoBuilder};
use elara_core::Value;
use elara_interp::{RuntimeResult, execute_proto};

use crate::{JitEntryStatus, JitRuntime, JitRuntimeMode};

#[test]
fn jit_equivalence_arithmetic_hot_path_matches_interpreter() {
    let proto = arithmetic_proto();
    let mut runtime = JitRuntime::new(JitRuntimeMode::Always);

    assert_equivalent(&proto, &mut runtime);
    assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Compiled);
    assert_eq!(runtime.stats().jit_runs, 1);
    assert_eq!(runtime.stats().fallback_runs, 0);
}

#[test]
fn jit_equivalence_table_path_falls_back_and_matches_interpreter() {
    let proto = table_proto();
    let mut runtime = JitRuntime::new(JitRuntimeMode::Always);

    assert_equivalent(&proto, &mut runtime);
    assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Unsupported);
    assert_eq!(runtime.stats().jit_runs, 0);
    assert_eq!(runtime.stats().fallback_runs, 1);
}

#[test]
fn jit_equivalence_lua_call_path_falls_back_and_matches_interpreter() {
    let proto = lua_call_proto();
    let mut runtime = JitRuntime::new(JitRuntimeMode::Always);

    assert_equivalent(&proto, &mut runtime);
    assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Unsupported);
    assert_eq!(runtime.stats().jit_runs, 0);
    assert_eq!(runtime.stats().fallback_runs, 1);
}

#[test]
fn jit_equivalence_yield_path_disables_jit_and_matches_interpreter_error() {
    let proto = yield_proto();
    let mut runtime = JitRuntime::new(JitRuntimeMode::Always);

    assert_equivalent(&proto, &mut runtime);
    assert_eq!(runtime.entry_status(&proto), JitEntryStatus::Unsupported);
    assert_eq!(runtime.stats().jit_runs, 0);
    assert_eq!(runtime.stats().fallback_runs, 1);
}

fn assert_equivalent(proto: &Proto, runtime: &mut JitRuntime) {
    assert_eq!(
        comparable(runtime.execute(proto)),
        comparable(execute_proto(proto))
    );
}

fn comparable(result: RuntimeResult<Vec<Value>>) -> Result<Vec<Value>, String> {
    result.map_err(|error| error.to_string())
}

fn arithmetic_proto() -> Proto {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let left = builder.add_constant(Value::integer(10));
    let right = builder.add_constant(Value::integer(4));
    builder.emit_abx(Op::LoadK, 0, u64::from(left));
    builder.emit_abx(Op::LoadK, 1, u64::from(right));
    builder.emit_abc(Op::Sub, 2, 0, 1);
    builder.emit_abc(Op::AddInt, 2, 2, 3);
    builder.emit_abc(Op::Return, 2, 1, 0);
    builder.finish()
}

fn table_proto() -> Proto {
    let mut builder = ProtoBuilder::new().with_signature(3, 0, false);
    let value = builder.add_constant(Value::integer(42));
    builder.emit_abc(Op::NewTable, 0, 1, 0);
    builder.emit_abx(Op::LoadK, 1, u64::from(value));
    builder.emit_abc(Op::SetIndex, 0, 1, 1);
    builder.emit_abc(Op::GetIndex, 2, 0, 1);
    builder.emit_abc(Op::Return, 2, 1, 0);
    builder.finish()
}

fn lua_call_proto() -> Proto {
    let mut child = ProtoBuilder::new().with_signature(1, 0, false);
    let value = child.add_constant(Value::integer(42));
    child.emit_abx(Op::LoadK, 0, u64::from(value));
    child.emit_abc(Op::Return, 0, 1, 0);

    let mut parent = ProtoBuilder::new().with_signature(1, 0, false);
    let child_index = parent.add_child(child.finish());
    parent.emit_abx(Op::Closure, 0, u64::from(child_index));
    parent.emit_abc(Op::Call, 0, 1, 1);
    parent.emit_abc(Op::Return, 0, 1, 0);
    parent.finish()
}

fn yield_proto() -> Proto {
    let mut builder = ProtoBuilder::new().with_signature(1, 0, false);
    let value = builder.add_constant(Value::integer(7));
    builder.emit_abx(Op::LoadK, 0, u64::from(value));
    builder.emit_abc(Op::Yield, 0, 1, 0);
    builder.finish()
}
