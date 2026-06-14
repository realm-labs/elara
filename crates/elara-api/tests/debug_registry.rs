use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn debug_getregistry_returns_mutable_runtime_registry() {
    let profile = StdLibProfile::Custom([StdLib::Debug].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local registry = debug.getregistry()\nregistry.answer = 42\nlocal again = debug.getregistry()\nreturn again.answer",
            &profile,
        ),
        Ok(vec![Value::integer(42)])
    );
}
