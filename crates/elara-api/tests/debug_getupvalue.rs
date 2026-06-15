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
fn debug_getupvalue_returns_lua_upvalue_name() {
    let values = eval_simple_source_with_stdlib(
        SourceId::new(0),
        "local x = 41
         local function answer()
           return x + 1
         end
         return debug.getupvalue(answer, 1)",
        &profile(),
    )
    .expect("debug.getupvalue should return a name");

    assert_eq!(values.len(), 2);
    assert!(values[0].is_string());
    assert_eq!(values[1], Value::integer(41));
}

#[test]
fn debug_getupvalue_returns_nil_for_absent_or_native_upvalues() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local x = 1
             local function answer()
               return x
             end
             local missing = debug.getupvalue(answer, 2)
             local native = debug.getupvalue(print, 1)
             return missing, native",
            &profile(),
        ),
        Ok(vec![Value::nil(), Value::nil()])
    );
}

#[test]
fn debug_getupvalue_rejects_bad_arguments() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local ok1 = pcall(debug.getupvalue, 1, 1)
             local ok2 = pcall(debug.getupvalue, print, false)
             return ok1, ok2",
            &profile(),
        ),
        Ok(vec![Value::boolean(false), Value::boolean(false)])
    );
}
