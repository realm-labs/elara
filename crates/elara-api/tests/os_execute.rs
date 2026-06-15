use elara_api::eval_simple_source_with_stdlib;
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn os_execute_reports_shell_availability() {
    let profile = StdLibProfile::Custom([StdLib::Os].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(SourceId::new(0), "return os.execute()", &profile),
        Ok(vec![Value::boolean(true)])
    );
}

#[test]
fn os_execute_reports_command_exit_status() {
    let profile = StdLibProfile::Custom([StdLib::Os].into_iter().collect());

    let success = eval_simple_source_with_stdlib(SourceId::new(0), success_source(), &profile)
        .expect("successful os.execute should return status values");
    assert_eq!(success.len(), 3);
    assert_eq!(success[0], Value::boolean(true));
    assert!(success[1].is_string());
    assert_eq!(success[2], Value::integer(0));

    let failure = eval_simple_source_with_stdlib(SourceId::new(0), failure_source(), &profile)
        .expect("failed os.execute should return status values");
    assert_eq!(failure.len(), 3);
    assert_eq!(failure[0], Value::nil());
    assert!(failure[1].is_string());
    assert_eq!(failure[2], Value::integer(7));
}

#[cfg(windows)]
fn success_source() -> &'static str {
    "return os.execute('exit /B 0')"
}

#[cfg(not(windows))]
fn success_source() -> &'static str {
    "return os.execute('exit 0')"
}

#[cfg(windows)]
fn failure_source() -> &'static str {
    "return os.execute('exit /B 7')"
}

#[cfg(not(windows))]
fn failure_source() -> &'static str {
    "return os.execute('exit 7')"
}
