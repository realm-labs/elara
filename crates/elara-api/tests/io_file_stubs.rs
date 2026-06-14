use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn io_close_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.close()", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn io_flush_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.flush()", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn io_input_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.input('file.txt')", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn io_output_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.output('file.txt')", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn io_tmpfile_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.tmpfile()", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn io_popen_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "return io.popen('echo hi', 'r')",
            &profile,
        ),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn io_read_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.read('*l')", &profile),
        Ok(vec![Value::nil()])
    );
}

#[test]
fn io_write_reports_unsupported_file_handles() {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return io.write('hello', 7)", &profile),
        Ok(vec![Value::nil()])
    );
}
