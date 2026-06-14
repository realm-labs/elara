//! Executable coroutine-library natives.

use elara_core::{ThreadStatus, Value};

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable coroutine-library functions currently implemented.
pub const COROUTINE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "create"),
        coroutine_create,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "status"),
        coroutine_status,
    ),
];

fn coroutine_create(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let function = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !function.is_closure() {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "function",
        }
        .into());
    }
    Ok(vec![runtime.create_coroutine(function)?])
}

fn coroutine_status(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let thread = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !thread.is_thread() {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "thread",
        }
        .into());
    }
    let status = runtime.thread_status(thread)?;
    Ok(vec![
        runtime.intern_short_string(status_name(status).as_bytes())?,
    ])
}

fn status_name(status: ThreadStatus) -> &'static str {
    match status {
        ThreadStatus::Runnable | ThreadStatus::Suspended => "suspended",
        ThreadStatus::Running => "running",
        ThreadStatus::Dead => "dead",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use elara_core::{ThreadStatus, Value};

    use super::{COROUTINE_NATIVE_FUNCTIONS, coroutine_create, coroutine_status};
    use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        statuses: BTreeMap<u32, ThreadStatus>,
    }

    impl TestRuntime {
        fn push_thread(&mut self, status: ThreadStatus) -> Value {
            let index = u32::try_from(self.statuses.len()).expect("test thread index fits in u32");
            self.statuses.insert(index, status);
            Value::thread_index(index)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Ok(Value::table_index(index))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_table_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }

        fn create_coroutine(&mut self, function: Value) -> Result<Value, NativeError> {
            if !function.is_closure() {
                return Err(NativeErrorKind::TypeError {
                    index: 1,
                    expected: "function",
                }
                .into());
            }
            Ok(self.push_thread(ThreadStatus::Runnable))
        }

        fn thread_status(&self, thread: Value) -> Result<ThreadStatus, NativeError> {
            let index = thread.as_thread_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "thread",
            })?;
            self.statuses.get(&index).copied().ok_or_else(|| {
                NativeErrorKind::RuntimeError {
                    message: "unknown coroutine".into(),
                }
                .into()
            })
        }
    }

    #[test]
    fn coroutine_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = COROUTINE_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "status")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "create")));
    }

    #[test]
    fn coroutine_create_returns_runnable_thread() {
        let mut runtime = TestRuntime::default();

        let values =
            coroutine_create(&mut runtime, &[Value::closure_index(3)]).expect("create should pass");

        assert_eq!(values.len(), 1);
        assert!(values[0].is_thread());
        assert_eq!(
            runtime.thread_status(values[0]).expect("thread has status"),
            ThreadStatus::Runnable
        );
    }

    #[test]
    fn coroutine_status_reports_thread_status_names() {
        let mut runtime = TestRuntime::default();
        let runnable = runtime.push_thread(ThreadStatus::Runnable);
        let suspended = runtime.push_thread(ThreadStatus::Suspended);
        let running = runtime.push_thread(ThreadStatus::Running);
        let dead = runtime.push_thread(ThreadStatus::Dead);

        let values = [
            (runnable, b"suspended".as_slice()),
            (suspended, b"suspended".as_slice()),
            (running, b"running".as_slice()),
            (dead, b"dead".as_slice()),
        ];
        for (thread, expected) in values {
            let status = coroutine_status(&mut runtime, &[thread]).expect("status should pass");
            assert_eq!(runtime.short_string_bytes(status[0]), Some(expected));
        }
    }

    #[test]
    fn coroutine_status_requires_thread_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            coroutine_status(&mut runtime, &[Value::nil()])
                .expect_err("non-thread should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "thread"
            }
        );
    }

    #[test]
    fn coroutine_create_requires_function_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            coroutine_create(&mut runtime, &[Value::nil()])
                .expect_err("non-function should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "function"
            }
        );
    }
}
