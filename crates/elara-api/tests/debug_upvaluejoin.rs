use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

fn profile() -> StdLibProfile {
    StdLibProfile::Custom([StdLib::Base, StdLib::Debug].into_iter().collect())
}

#[test]
fn debug_upvaluejoin_makes_target_share_source_upvalue() {
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
             local joined = debug.upvaluejoin(first, 1, second, 1)
             local name = debug.setupvalue(second, 1, 42)
             local first_id = debug.upvalueid(first, 1)
             local second_id = debug.upvalueid(second, 1)
             return first(), second(), rawequal(first_id, second_id)",
            &profile(),
        ),
        Ok(vec![
            Value::integer(42),
            Value::integer(42),
            Value::boolean(true),
        ])
    );
}

#[test]
fn debug_upvaluejoin_rejects_invalid_or_native_targets() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local x = 1
             local function read()
               return x
             end
             local missing = pcall(debug.upvaluejoin, read, 2, read, 1)
             local native_target = pcall(debug.upvaluejoin, print, 1, read, 1)
             local native_source = pcall(debug.upvaluejoin, read, 1, print, 1)
             return missing, native_target, native_source",
            &profile(),
        ),
        Ok(vec![
            Value::boolean(false),
            Value::boolean(false),
            Value::boolean(false),
        ])
    );
}
