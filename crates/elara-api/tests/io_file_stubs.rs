use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

fn assert_unsupported_file_result(source: &str) {
    let profile = StdLibProfile::Custom([StdLib::Io].into_iter().collect());
    let values = eval_simple_source_with_stdlib(SourceId::new(0), source, &profile)
        .expect("io stub should return unsupported-file result values");

    assert_eq!(values.len(), 2);
    assert_eq!(values[0], Value::nil());
    assert!(values[1].is_string());
}

#[test]
fn io_close_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.close()");
}

#[test]
fn io_flush_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.flush()");
}

#[test]
fn io_input_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.input('file.txt')");
}

#[test]
fn io_lines_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.lines('file.txt', '*l')");
}

#[test]
fn io_output_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.output('file.txt')");
}

#[test]
fn io_tmpfile_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.tmpfile()");
}

#[test]
fn io_popen_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.popen('echo hi', 'r')");
}

#[test]
fn io_read_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.read('*l')");
}

#[test]
fn io_write_reports_unsupported_file_handles() {
    assert_unsupported_file_result("return io.write('hello', 7)");
}
