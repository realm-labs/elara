use std::sync::{Arc, Mutex};

use elara_api::eval_simple_source_with_stdlib;
use elara_bytecode::{Instr, Op, ProtoBuilder};
use elara_core::{SourceId, Value};
use elara_interp::execute_proto_with_environment;
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn debug_gethook_returns_nil_without_installed_hook() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return debug.gethook()", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn debug_sethook_clears_hooks() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return debug.sethook()", &profile,),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn debug_sethook_installs_and_gethook_returns_metadata() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());
    let mut environment = elara_api::stdlib::runtime_environment_for_stdlib(&profile);
    environment.register_simple_native_global("hook", |_args| Ok(Vec::new()));
    let mut builder = ProtoBuilder::new().with_signature(5, 0, false);
    let debug = builder.add_string_constant("debug");
    let sethook = builder.add_string_constant("sethook");
    let gethook = builder.add_string_constant("gethook");
    let hook = builder.add_string_constant("hook");
    let mask = builder.add_string_constant("lrcx");
    let count = builder.add_constant(Value::integer(5));

    builder.emit_abx(Op::GetEnv, 0, u64::from(debug));
    builder.emit_abx(Op::LoadString, 1, u64::from(sethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abx(Op::GetEnv, 2, u64::from(hook));
    builder.emit_abx(Op::LoadString, 3, u64::from(mask));
    builder.emit_abx(Op::LoadK, 4, u64::from(count));
    builder.emit_abc(Op::Call, 1, 4, 0);
    builder.emit_abx(Op::LoadString, 1, u64::from(gethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abc(Op::Call, 1, 1, 3);
    builder.emit_abc(Op::Return, 1, 3, 0);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("debug hook bytecode should execute");

    assert_eq!(output.values.len(), 3);
    assert!(output.values[0].is_closure());
    assert!(output.values[1].is_string());
    assert_eq!(output.values[2], Value::integer(5));
}

#[test]
fn debug_sethook_clears_installed_hook() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());
    let mut environment = elara_api::stdlib::runtime_environment_for_stdlib(&profile);
    environment.register_simple_native_global("hook", |_args| Ok(Vec::new()));
    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let debug = builder.add_string_constant("debug");
    let sethook = builder.add_string_constant("sethook");
    let gethook = builder.add_string_constant("gethook");
    let hook = builder.add_string_constant("hook");
    let mask = builder.add_string_constant("c");

    builder.emit_abx(Op::GetEnv, 0, u64::from(debug));
    builder.emit_abx(Op::LoadString, 1, u64::from(sethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abx(Op::GetEnv, 2, u64::from(hook));
    builder.emit_abx(Op::LoadString, 3, u64::from(mask));
    builder.emit_abc(Op::Call, 1, 3, 1);
    builder.emit_abx(Op::LoadString, 1, u64::from(sethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abc(Op::Call, 1, 1, 1);
    builder.emit_abx(Op::LoadString, 1, u64::from(gethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abc(Op::Call, 1, 1, 1);
    builder.emit_abc(Op::Return, 1, 1, 0);

    assert_eq!(
        execute_proto_with_environment(&builder.finish(), environment)
            .expect("debug hook bytecode should execute")
            .values,
        vec![Value::nil()]
    );
}

#[test]
fn debug_sethook_dispatches_call_and_return_events() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());
    let mut environment = elara_api::stdlib::runtime_environment_for_stdlib(&profile);
    let events = Arc::new(Mutex::new(Vec::new()));
    let hook_events = Arc::clone(&events);
    environment.register_native_global("hook", move |context, args| {
        let event = args
            .first()
            .and_then(|value| context.string_bytes(*value))
            .expect("debug hook event must be a runtime string")
            .to_vec();
        let line_is_nil = args.get(1).is_none_or(|value| value.is_nil());
        hook_events
            .lock()
            .expect("hook event list lock must not be poisoned")
            .push((event, line_is_nil));
        Ok(Vec::new())
    });
    environment.register_simple_native_global("target", |_args| Ok(vec![Value::integer(42)]));

    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let debug = builder.add_string_constant("debug");
    let sethook = builder.add_string_constant("sethook");
    let hook = builder.add_string_constant("hook");
    let mask = builder.add_string_constant("cr");
    let target = builder.add_string_constant("target");

    builder.emit_abx(Op::GetEnv, 0, u64::from(debug));
    builder.emit_abx(Op::LoadString, 1, u64::from(sethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abx(Op::GetEnv, 2, u64::from(hook));
    builder.emit_abx(Op::LoadString, 3, u64::from(mask));
    builder.emit_abc(Op::Call, 1, 3, 1);
    builder.emit_abx(Op::GetEnv, 1, u64::from(target));
    builder.emit_abc(Op::Call, 1, 1, 1);
    builder.emit_abc(Op::Return, 1, 1, 0);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("debug hook bytecode should execute");

    assert_eq!(output.values, vec![Value::integer(42)]);
    assert_eq!(
        *events
            .lock()
            .expect("hook event list lock must not be poisoned"),
        vec![
            (b"return".to_vec(), true),
            (b"call".to_vec(), true),
            (b"return".to_vec(), true),
        ]
    );
}

#[test]
fn debug_sethook_dispatches_line_events() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());
    let mut environment = elara_api::stdlib::runtime_environment_for_stdlib(&profile);
    let events = Arc::new(Mutex::new(Vec::new()));
    let hook_events = Arc::clone(&events);
    environment.register_native_global("hook", move |context, args| {
        let event = args
            .first()
            .and_then(|value| context.string_bytes(*value))
            .expect("debug hook event must be a runtime string")
            .to_vec();
        let line = args.get(1).and_then(|value| value.as_integer());
        hook_events
            .lock()
            .expect("hook event list lock must not be poisoned")
            .push((event, line));
        Ok(Vec::new())
    });
    environment.register_simple_native_global("target", |_args| Ok(vec![Value::integer(42)]));

    let mut builder = ProtoBuilder::new().with_signature(4, 0, false);
    let debug = builder.add_string_constant("debug");
    let sethook = builder.add_string_constant("sethook");
    let hook = builder.add_string_constant("hook");
    let mask = builder.add_string_constant("l");
    let target = builder.add_string_constant("target");

    builder.emit_line(Instr::abx(Op::GetEnv, 0, u64::from(debug)), 1);
    builder.emit_line(Instr::abx(Op::LoadString, 1, u64::from(sethook)), 1);
    builder.emit_line(Instr::abc(Op::GetTable, 1, 0, 1), 1);
    builder.emit_line(Instr::abx(Op::GetEnv, 2, u64::from(hook)), 1);
    builder.emit_line(Instr::abx(Op::LoadString, 3, u64::from(mask)), 1);
    builder.emit_line(Instr::abc(Op::Call, 1, 3, 1), 1);
    builder.emit_line(Instr::abx(Op::GetEnv, 1, u64::from(target)), 20);
    builder.emit_line(Instr::abc(Op::Call, 1, 1, 1), 20);
    builder.emit_line(Instr::abc(Op::Return, 1, 1, 0), 21);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("debug line hook bytecode should execute");

    assert_eq!(output.values, vec![Value::integer(42)]);
    assert_eq!(
        *events
            .lock()
            .expect("hook event list lock must not be poisoned"),
        vec![(b"line".to_vec(), Some(20)), (b"line".to_vec(), Some(21)),]
    );
}

#[test]
fn debug_sethook_dispatches_count_events() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());
    let mut environment = elara_api::stdlib::runtime_environment_for_stdlib(&profile);
    let events = Arc::new(Mutex::new(Vec::new()));
    let hook_events = Arc::clone(&events);
    environment.register_native_global("hook", move |context, args| {
        let event = args
            .first()
            .and_then(|value| context.string_bytes(*value))
            .expect("debug hook event must be a runtime string")
            .to_vec();
        let line_is_nil = args.get(1).is_none_or(|value| value.is_nil());
        hook_events
            .lock()
            .expect("hook event list lock must not be poisoned")
            .push((event, line_is_nil));
        Ok(Vec::new())
    });
    environment.register_simple_native_global("target", |_args| Ok(vec![Value::integer(42)]));

    let mut builder = ProtoBuilder::new().with_signature(5, 0, false);
    let debug = builder.add_string_constant("debug");
    let sethook = builder.add_string_constant("sethook");
    let hook = builder.add_string_constant("hook");
    let mask = builder.add_string_constant("");
    let count = builder.add_constant(Value::integer(2));
    let target = builder.add_string_constant("target");

    builder.emit_abx(Op::GetEnv, 0, u64::from(debug));
    builder.emit_abx(Op::LoadString, 1, u64::from(sethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abx(Op::GetEnv, 2, u64::from(hook));
    builder.emit_abx(Op::LoadString, 3, u64::from(mask));
    builder.emit_abx(Op::LoadK, 4, u64::from(count));
    builder.emit_abc(Op::Call, 1, 4, 1);
    builder.emit_abx(Op::GetEnv, 1, u64::from(target));
    builder.emit_abc(Op::Call, 1, 1, 1);
    builder.emit_abc(Op::Return, 1, 1, 0);

    let output = execute_proto_with_environment(&builder.finish(), environment)
        .expect("debug count hook bytecode should execute");

    assert_eq!(output.values, vec![Value::integer(42)]);
    assert_eq!(
        *events
            .lock()
            .expect("hook event list lock must not be poisoned"),
        vec![(b"count".to_vec(), true)]
    );
}
