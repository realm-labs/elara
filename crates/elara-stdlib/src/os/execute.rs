use std::process::{Command, ExitStatus};

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub(super) fn os_execute(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    match args.first().copied() {
        None => Ok(vec![Value::boolean(shell_available())]),
        Some(value) if value.is_nil() => Ok(vec![Value::boolean(shell_available())]),
        Some(command) => execute_command(runtime, command),
    }
}

fn execute_command(
    runtime: &mut dyn NativeRuntime,
    command: Value,
) -> Result<Vec<Value>, NativeError> {
    let command = runtime
        .string_bytes(command)
        .ok_or(NativeErrorKind::TypeError {
            index: 1,
            expected: "string",
        })?;
    let command = std::str::from_utf8(command).map_err(|_| NativeErrorKind::TypeError {
        index: 1,
        expected: "utf-8 string",
    })?;

    match shell_command(command).status() {
        Ok(status) => command_status(runtime, status),
        Err(error) => Ok(vec![
            Value::nil(),
            runtime.intern_string(error.to_string().as_bytes())?,
        ]),
    }
}

fn shell_available() -> bool {
    shell_command("").status().is_ok()
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("cmd");
    process.arg("/C").arg(command);
    process
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("sh");
    process.arg("-c").arg(command);
    process
}

fn command_status(
    runtime: &mut dyn NativeRuntime,
    status: ExitStatus,
) -> Result<Vec<Value>, NativeError> {
    let (success, what, code) = status_tuple(status);
    Ok(vec![
        if success {
            Value::boolean(true)
        } else {
            Value::nil()
        },
        runtime.intern_string(what)?,
        Value::integer(i64::from(code)),
    ])
}

fn status_tuple(status: ExitStatus) -> (bool, &'static [u8], i32) {
    if let Some(code) = status.code() {
        return (code == 0, b"exit", code);
    }

    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return (false, b"signal", signal);
    }

    (false, b"exit", -1)
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{NativeError, NativeErrorKind, NativeRuntime};

    use super::os_execute;

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::closure_index(index)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn intern_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    #[test]
    fn os_execute_without_command_reports_shell_availability() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            os_execute(&mut runtime, &[]).expect("os.execute should pass"),
            vec![Value::boolean(true)]
        );
        assert_eq!(
            os_execute(&mut runtime, &[Value::nil()]).expect("os.execute(nil) should pass"),
            vec![Value::boolean(true)]
        );
    }

    #[test]
    fn os_execute_reports_success_status_tuple() {
        let mut runtime = TestRuntime::default();
        let command = runtime.push_string(success_command());

        let values = os_execute(&mut runtime, &[command]).expect("os.execute should pass");

        assert_eq!(values[0], Value::boolean(true));
        assert_eq!(
            runtime.short_string_bytes(values[1]),
            Some(b"exit".as_slice())
        );
        assert_eq!(values[2], Value::integer(0));
    }

    #[test]
    fn os_execute_reports_failure_status_tuple() {
        let mut runtime = TestRuntime::default();
        let command = runtime.push_string(failure_command());

        let values = os_execute(&mut runtime, &[command]).expect("os.execute should pass");

        assert_eq!(values[0], Value::nil());
        assert_eq!(
            runtime.short_string_bytes(values[1]),
            Some(b"exit".as_slice())
        );
        assert_eq!(values[2], Value::integer(7));
    }

    #[test]
    fn os_execute_validates_command_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            os_execute(&mut runtime, &[Value::integer(1)])
                .expect_err("command must be a string")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
    }

    #[cfg(windows)]
    fn success_command() -> &'static [u8] {
        b"exit /B 0"
    }

    #[cfg(not(windows))]
    fn success_command() -> &'static [u8] {
        b"exit 0"
    }

    #[cfg(windows)]
    fn failure_command() -> &'static [u8] {
        b"exit /B 7"
    }

    #[cfg(not(windows))]
    fn failure_command() -> &'static [u8] {
        b"exit 7"
    }
}
