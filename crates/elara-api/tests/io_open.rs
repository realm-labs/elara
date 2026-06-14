use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn io_open_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "return io.open('file.txt', 'r')",
            &profile,
        ),
        Ok(vec![Value::nil()])
    );
}
