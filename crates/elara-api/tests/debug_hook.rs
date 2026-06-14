use elara_api::eval_simple_source_with_stdlib;
use elara_bytecode::{Op, ProtoBuilder};
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
    let environment = elara_api::stdlib::runtime_environment_for_stdlib(&profile);
    let mut builder = ProtoBuilder::new().with_signature(5, 0, false);
    let debug = builder.add_string_constant("debug");
    let sethook = builder.add_string_constant("sethook");
    let gethook = builder.add_string_constant("gethook");
    let mask = builder.add_string_constant("lrcx");
    let count = builder.add_constant(Value::integer(5));

    builder.emit_abx(Op::GetEnv, 0, u64::from(debug));
    builder.emit_abx(Op::LoadString, 1, u64::from(sethook));
    builder.emit_abc(Op::GetTable, 1, 0, 1);
    builder.emit_abc(Op::Move, 2, 1, 0);
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
    let profile = StdLibProfile::Custom([StdLib::Base, StdLib::Debug].into_iter().collect());
    let source = "\
if pcall(debug.sethook, debug.sethook, 'c') then end
if pcall(debug.sethook) then end
return debug.gethook()
";

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), source, &profile),
        Ok(vec![Value::nil()])
    );
}
