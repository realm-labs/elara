use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn debug_traceback_returns_standard_header() {
    let profile = StdLibProfile::Custom([StdLib::Debug, StdLib::String].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local t = debug.traceback()\nreturn string.len(t), string.byte(t, 1), string.byte(t, 7), string.byte(t, 16)",
            &profile,
        ),
        Ok(vec![
            Value::integer(16),
            Value::integer(115),
            Value::integer(116),
            Value::integer(58),
        ])
    );
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local t = debug.traceback('boom')\nreturn string.len(t), string.byte(t, 1), string.byte(t, 5), string.byte(t, 6), string.byte(t, 21)",
            &profile,
        ),
        Ok(vec![
            Value::integer(21),
            Value::integer(98),
            Value::integer(10),
            Value::integer(115),
            Value::integer(58),
        ])
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
