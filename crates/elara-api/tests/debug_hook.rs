use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn debug_gethook_returns_nil_without_installed_hook() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return debug.gethook()", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn debug_sethook_clears_hooks() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return debug.sethook()", &profile,),
        Ok(vec![Value::nil()])
    );
}
