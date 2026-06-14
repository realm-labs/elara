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
fn debug_setupvalue_updates_lua_upvalue_and_returns_name() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local x = 41
             local function read()
               return x
             end
             local name = debug.setupvalue(read, 1, 42)
             return string.byte(name, 1), read()",
            &profile(),
        ),
        Ok(vec![Value::integer(120), Value::integer(42)])
    );
}

#[test]
fn debug_setupvalue_returns_nil_for_absent_or_native_upvalues() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local x = 1
             local function read()
               return x
             end
             local missing = debug.setupvalue(read, 2, 9)
             local native = debug.setupvalue(print, 1, 9)
             return missing, native",
            &profile(),
        ),
        Ok(vec![Value::nil(), Value::nil()])
    );
}

#[test]
fn debug_setupvalue_rejects_bad_arguments() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local ok1 = pcall(debug.setupvalue, 1, 1, 1)
             local ok2 = pcall(debug.setupvalue, print, false, 1)
             local ok3 = pcall(debug.setupvalue, print, 1)
             return ok1, ok2, ok3",
            &profile(),
        ),
        Ok(vec![
            Value::boolean(false),
            Value::boolean(false),
            Value::boolean(false),
        ])
    );
}
