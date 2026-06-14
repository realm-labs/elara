use std::sync::{Arc, Mutex};

use elara_api::eval_simple_source_with_stdlib;
use elara_bytecode::{Instr, Op, ProtoBuilder};
use elara_core::{SourceId, Value};
use elara_interp::{RuntimeEnvironment, execute_proto_with_environment};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn debug_traceback_materializes_current_frame() {
    let bytes = capture_traceback(None);
    assert_eq!(bytes, b"stack traceback:\n\ttrace.lua:42: in main chunk");
}

#[test]
fn debug_traceback_prefixes_message_before_frames() {
    let bytes = capture_traceback(Some("boom"));
    assert_eq!(
        bytes,
        b"boom\nstack traceback:\n\ttrace.lua:42: in main chunk"
    );
}

#[test]
fn debug_traceback_preserves_non_string_messages() {
    let profile = StdLibProfile::Custom([StdLib::Base, StdLib::Debug].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local message = {}\nreturn rawequal(debug.traceback(message), message)",
            &profile,
        ),
        Ok(vec![Value::boolean(true)])
    );
}

fn capture_traceback(message: Option<&str>) -> Vec<u8> {
    let captured = Arc::new(Mutex::new(None));
    let capture_target = Arc::clone(&captured);
    let mut environment = debug_environment();
    environment.register_native_global("capture", move |context, args| {
        let bytes = args
            .first()
            .and_then(|value| context.string_bytes(*value))
            .expect("traceback argument must be a runtime string")
            .to_vec();
        *capture_target
            .lock()
            .expect("traceback capture lock must not be poisoned") = Some(bytes);
        Ok(Vec::new())
    });
    execute_proto_with_environment(&traceback_proto(message), environment)
        .expect("debug traceback bytecode should execute");
    captured
        .lock()
        .expect("traceback capture lock must not be poisoned")
        .clone()
        .expect("traceback should be captured")
}

fn debug_environment() -> RuntimeEnvironment {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());
    elara_api::stdlib::runtime_environment_for_stdlib(&profile)
}

fn traceback_proto(message: Option<&str>) -> elara_bytecode::Proto {
    let mut builder = ProtoBuilder::new()
        .with_signature(4, 0, false)
        .with_source_name("trace.lua");
    let debug = builder.add_string_constant("debug");
    let traceback = builder.add_string_constant("traceback");
    let capture = builder.add_string_constant("capture");

    builder.emit_line(Instr::abx(Op::GetEnv, 0, u64::from(debug)), 1);
    builder.emit_line(Instr::abx(Op::LoadString, 1, u64::from(traceback)), 1);
    builder.emit_line(Instr::abc(Op::GetTable, 1, 0, 1), 1);
    let arg_count = if let Some(message) = message {
        let message = builder.add_string_constant(message);
        builder.emit_line(Instr::abx(Op::LoadString, 2, u64::from(message)), 41);
        2
    } else {
        1
    };
    builder.emit_line(Instr::abc(Op::Call, 1, arg_count, 1), 42);
    builder.emit_line(Instr::abx(Op::GetEnv, 0, u64::from(capture)), 43);
    builder.emit_line(Instr::abc(Op::Call, 0, 2, 1), 43);
    builder.emit_line(Instr::abc(Op::Return, 0, 1, 0), 44);
    builder.finish()
}
