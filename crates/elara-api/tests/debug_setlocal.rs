use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

fn profile() -> StdLibProfile {
    StdLibProfile::Custom([StdLib::Base, StdLib::Debug].into_iter().collect())
}

#[test]
fn debug_setlocal_updates_lua_local_value() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function probe()
               local x = 1
               local _ = debug.setlocal(1, 1, 42)
               return x
             end
             return probe()",
            &profile(),
        ),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn debug_setlocal_returns_nil_for_absent_locals_without_mutation() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function probe()
               local x = 1
               local _ = debug.setlocal(1, 2, 42)
               return x
             end
             return probe()",
            &profile(),
        ),
        Ok(vec![Value::integer(1)])
    );
}

#[test]
fn debug_setlocal_rejects_bad_arguments_and_out_of_range_levels() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local ok1 = pcall(debug.setlocal, false, 1, 2)
             local ok2 = pcall(debug.setlocal, 1, false, 2)
             local ok3 = pcall(debug.setlocal, 1, 1)
             local ok4 = pcall(debug.setlocal, 99, 1, 2)
             return ok1, ok2, ok3, ok4",
            &profile(),
        ),
        Ok(vec![
            Value::boolean(false),
            Value::boolean(false),
            Value::boolean(false),
            Value::boolean(false),
        ])
    );
}
