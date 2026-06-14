use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

fn profile() -> StdLibProfile {
    StdLibProfile::Custom(
        [StdLib::Base, StdLib::Debug, StdLib::String]
            .into_iter()
            .collect(),
    )
}

#[test]
fn debug_getinfo_reports_lua_stack_frame_fields() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local info = debug.getinfo(1, 'Sut')
             local what = string.byte(info.what, 1)
             return what, info.nparams, info.isvararg, info.istailcall, info.extraargs",
            &profile(),
        ),
        Ok(vec![
            Value::integer(109),
            Value::integer(0),
            Value::boolean(false),
            Value::boolean(false),
            Value::integer(0),
        ])
    );
}

#[test]
fn debug_getinfo_reports_function_targets() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function probe() return 1 end
             local info = debug.getinfo(probe, 'Su')
             return string.byte(info.what, 1), info.nparams, info.isvararg",
            &profile(),
        ),
        Ok(vec![
            Value::integer(76),
            Value::integer(0),
            Value::boolean(false)
        ])
    );

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local info = debug.getinfo(print, 'Su')
             return string.byte(info.what, 1), info.nparams, info.isvararg",
            &profile(),
        ),
        Ok(vec![
            Value::integer(67),
            Value::integer(0),
            Value::boolean(true)
        ])
    );
}

#[test]
fn debug_getinfo_returns_nil_for_unavailable_levels() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "return rawequal(debug.getinfo(1000), nil), rawequal(debug.getinfo(-1), nil)",
            &profile(),
        ),
        Ok(vec![Value::boolean(true), Value::boolean(true)])
    );
}

#[test]
fn debug_getinfo_honors_empty_option_string() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local info = debug.getinfo(1, '')
             return info.currentline, info.what, info.func",
            &profile(),
        ),
        Ok(vec![Value::nil(), Value::nil(), Value::nil()])
    );
}

#[test]
fn debug_getinfo_rejects_invalid_options() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local ok = pcall(debug.getinfo, 1, 'X')
             return ok",
            &profile(),
        ),
        Ok(vec![Value::boolean(false)])
    );
}
