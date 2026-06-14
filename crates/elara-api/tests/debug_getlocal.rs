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
fn debug_getlocal_returns_lua_local_name_and_value() {
    let values = eval_simple_source_with_stdlib(
        SourceId::new(0),
        "local function probe()
           local x = 42
           return debug.getlocal(1, 1)
         end
         return probe()",
        &profile(),
    )
    .expect("debug.getlocal should return a name/value pair");

    assert_eq!(values.len(), 1);
    assert!(values[0].is_string());
}

#[test]
fn debug_getlocal_returns_nil_for_absent_locals() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function probe()
               local x = 42
               return debug.getlocal(1, 2)
             end
             return probe()",
            &profile(),
        ),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn debug_getlocal_rejects_bad_arguments_and_out_of_range_levels() {
    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local ok1 = pcall(debug.getlocal, false, 1)
             local ok2 = pcall(debug.getlocal, 1, false)
             local ok3 = pcall(debug.getlocal, 99, 1)
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
