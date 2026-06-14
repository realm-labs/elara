use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

fn profile() -> StdLibProfile {
    StdLibProfile::Custom([StdLib::Base, StdLib::Debug].into_iter().collect())
}

#[test]
fn debug_upvalueid_returns_shared_identity_for_shared_upvalues() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local x = 41
             local function first()
               return x
             end
             local function second()
               return x
             end
             local first_id = debug.upvalueid(first, 1)
             local second_id = debug.upvalueid(second, 1)
             return rawequal(first_id, second_id)",
            &profile(),
        ),
        Ok(vec![Value::boolean(true)])
    );
}

#[test]
fn debug_upvalueid_distinguishes_different_upvalues() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local x = 1
             local y = 2
             local function first()
               return x
             end
             local function second()
               return y
             end
             local first_id = debug.upvalueid(first, 1)
             local second_id = debug.upvalueid(second, 1)
             return rawequal(first_id, second_id)",
            &profile(),
        ),
        Ok(vec![Value::boolean(false)])
    );
}

#[test]
fn debug_upvalueid_returns_nil_for_absent_or_native_upvalues() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local x = 1
             local function read()
               return x
             end
             local missing = debug.upvalueid(read, 2)
             local native = debug.upvalueid(print, 1)
             return missing, native",
            &profile(),
        ),
        Ok(vec![Value::nil(), Value::nil()])
    );
}

#[test]
fn debug_upvalueid_rejects_bad_arguments() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local ok1 = pcall(debug.upvalueid, 1, 1)
             local ok2 = pcall(debug.upvalueid, print, false)
             return ok1, ok2",
            &profile(),
        ),
        Ok(vec![Value::boolean(false), Value::boolean(false)])
    );
}
