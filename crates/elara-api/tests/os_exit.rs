use elara_api::{EvalError, eval_simple_source_with_stdlib};
use elara_core::SourceId;
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn os_exit_reports_unsupported_process_termination() {
    let profile = StdLibProfile::Custom([StdLib::Os].into_iter().collect());
    let error =
        eval_simple_source_with_stdlib(SourceId::new(0), "return os.exit(0, true)", &profile)
            .expect_err("os.exit should not terminate the host process");

    match error {
        EvalError::Runtime(error) => {
            assert_eq!(error.message(), "os.exit is not supported by this runtime");
        }
        EvalError::Diagnostics(diagnostics) => {
            panic!("expected runtime error, got diagnostics {diagnostics:?}");
        }
    }
}
