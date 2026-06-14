use elara_api::{EvalError, eval_simple_source_with_stdlib};
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn debug_getuservalue_returns_nil_for_current_values() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "return debug.getuservalue(), debug.getuservalue(1)",
            &profile,
        ),
        Ok(vec![Value::nil(), Value::nil()])
    );
}

#[test]
fn debug_setuservalue_rejects_non_userdata() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());

    let error = eval_simple_source_with_stdlib(
        SourceId::new(0),
        "return debug.setuservalue(1, 2)",
        &profile,
    )
    .expect_err("non-userdata should fail");

    match error {
        EvalError::Runtime(error) => {
            assert_eq!(error.message(), "bad argument #1 (userdata expected)");
        }
        EvalError::Diagnostics(diagnostics) => {
            panic!("unexpected diagnostics: {diagnostics:?}");
        }
    }
}
