use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn io_type_returns_nil_without_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.type(nil)", &profile),
        Ok(vec![Value::nil()])
    );
    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.type(1)", &profile),
        Ok(vec![Value::nil()])
    );
}
