//! Executable coroutine-library natives.

use elara_core::{ThreadStatus, Value};

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable coroutine-library functions currently implemented.
pub const COROUTINE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "close"),
        coroutine_close,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "create"),
        coroutine_create,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "isyieldable"),
        coroutine_isyieldable,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "resume"),
        coroutine_resume,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "running"),
        coroutine_running,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "status"),
        coroutine_status,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Coroutine, "yield"),
        coroutine_yield,
    ),
];

fn coroutine_close(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let thread = match args.first().copied() {
        Some(value) if !value.is_nil() => {
            if !value.is_thread() {
                return Err(NativeErrorKind::TypeError {
                    index: 1,
                    expected: "thread",
                }
                .into());
            }
            value
        }
        _ => runtime.running_thread()?.0,
    };
    match runtime.close_coroutine(thread)? {
        Ok(()) => Ok(vec![Value::boolean(true)]),
        Err(message) => Ok(vec![
            Value::boolean(false),
            runtime.intern_short_string(message.as_bytes())?,
        ]),
    }
}

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

fn coroutine_resume(
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
    let resume_args = args.get(1..).unwrap_or_default();
    match runtime.resume_coroutine(thread, resume_args)? {
        Ok(values) => {
            let mut results = Vec::with_capacity(values.len() + 1);
            results.push(Value::boolean(true));
            results.extend(values);
            Ok(results)
        }
        Err(message) => Ok(vec![
            Value::boolean(false),
            runtime.intern_short_string(message.as_bytes())?,
        ]),
    }
}

fn coroutine_yield(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    runtime.yield_coroutine(args)
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

fn coroutine_running(
    runtime: &mut dyn NativeRuntime,
    _args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let (thread, is_main) = runtime.running_thread()?;
    Ok(vec![thread, Value::boolean(is_main)])
}

fn coroutine_isyieldable(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let thread = match args.first().copied() {
        Some(value) if !value.is_nil() => {
            if !value.is_thread() {
                return Err(NativeErrorKind::TypeError {
                    index: 1,
                    expected: "thread",
                }
                .into());
            }
            value
        }
        _ => runtime.running_thread()?.0,
    };
    Ok(vec![Value::boolean(runtime.thread_is_yieldable(thread)?)])
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

    use super::{
        COROUTINE_NATIVE_FUNCTIONS, coroutine_close, coroutine_create, coroutine_isyieldable,
        coroutine_resume, coroutine_running, coroutine_status, coroutine_yield,
    };
    use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        statuses: BTreeMap<u32, ThreadStatus>,
        resume_calls: Vec<(Value, Vec<Value>)>,
        resume_results: BTreeMap<u32, Result<Vec<Value>, Box<str>>>,
        running: Option<(Value, bool)>,
        yield_result: Result<Vec<Value>, NativeError>,
        yield_calls: Vec<Vec<Value>>,
    }

    impl Default for TestRuntime {
        fn default() -> Self {
            Self {
                strings: Vec::new(),
                statuses: BTreeMap::new(),
                resume_calls: Vec::new(),
                resume_results: BTreeMap::new(),
                running: None,
                yield_result: Ok(Vec::new()),
                yield_calls: Vec::new(),
            }
        }
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

        fn close_coroutine(&mut self, thread: Value) -> Result<Result<(), Box<str>>, NativeError> {
            let index = thread.as_thread_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "thread",
            })?;
            match self.statuses.get_mut(&index) {
                Some(ThreadStatus::Runnable | ThreadStatus::Suspended | ThreadStatus::Dead) => {
                    self.statuses.insert(index, ThreadStatus::Dead);
                    Ok(Ok(()))
                }
                Some(ThreadStatus::Running) => Ok(Err("cannot close a running coroutine".into())),
                None => Err(NativeErrorKind::RuntimeError {
                    message: "unknown coroutine".into(),
                }
                .into()),
            }
        }

        fn resume_coroutine(
            &mut self,
            thread: Value,
            args: &[Value],
        ) -> Result<Result<Vec<Value>, Box<str>>, NativeError> {
            let index = thread.as_thread_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "thread",
            })?;
            self.resume_calls.push((thread, args.to_vec()));
            match self.statuses.get_mut(&index) {
                Some(ThreadStatus::Runnable | ThreadStatus::Suspended) => {
                    self.statuses.insert(index, ThreadStatus::Dead);
                    Ok(self
                        .resume_results
                        .remove(&index)
                        .unwrap_or_else(|| Ok(Vec::new())))
                }
                Some(ThreadStatus::Dead) => Ok(Err("cannot resume dead coroutine".into())),
                Some(ThreadStatus::Running) => {
                    Ok(Err("cannot resume non-suspended coroutine".into()))
                }
                None => Err(NativeErrorKind::RuntimeError {
                    message: "unknown coroutine".into(),
                }
                .into()),
            }
        }

        fn yield_coroutine(&mut self, args: &[Value]) -> Result<Vec<Value>, NativeError> {
            self.yield_calls.push(args.to_vec());
            self.yield_result.clone()
        }

        fn running_thread(&self) -> Result<(Value, bool), NativeError> {
            self.running.ok_or_else(|| {
                NativeErrorKind::RuntimeError {
                    message: "running thread is not registered".into(),
                }
                .into()
            })
        }

        fn thread_is_yieldable(&self, thread: Value) -> Result<bool, NativeError> {
            let index = thread.as_thread_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "thread",
            })?;
            Ok(index != 0
                && self
                    .statuses
                    .get(&index)
                    .copied()
                    .is_some_and(|status| status != ThreadStatus::Dead))
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

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "close")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "status")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "create")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "isyieldable")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "resume")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "running")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Coroutine, "yield")));
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
    fn coroutine_close_marks_closeable_thread_dead() {
        let mut runtime = TestRuntime::default();
        let thread = runtime.push_thread(ThreadStatus::Runnable);

        let values = coroutine_close(&mut runtime, &[thread]).expect("close should pass");

        assert_eq!(values, vec![Value::boolean(true)]);
        assert_eq!(
            runtime.thread_status(thread).expect("thread has status"),
            ThreadStatus::Dead
        );
    }

    #[test]
    fn coroutine_close_returns_false_and_error_message_on_runtime_close_failure() {
        let mut runtime = TestRuntime::default();
        let thread = runtime.push_thread(ThreadStatus::Running);

        let values = coroutine_close(&mut runtime, &[thread]).expect("close should pass");

        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::boolean(false));
        assert_eq!(
            runtime.short_string_bytes(values[1]),
            Some(b"cannot close a running coroutine".as_slice())
        );
    }

    #[test]
    fn coroutine_resume_prepends_true_to_return_values() {
        let mut runtime = TestRuntime::default();
        let thread = runtime.push_thread(ThreadStatus::Runnable);
        runtime
            .resume_results
            .insert(0, Ok(vec![Value::integer(42), Value::boolean(false)]));

        let values = coroutine_resume(&mut runtime, &[thread, Value::integer(7)])
            .expect("resume should pass");

        assert_eq!(
            values,
            vec![
                Value::boolean(true),
                Value::integer(42),
                Value::boolean(false)
            ]
        );
        assert_eq!(
            runtime.resume_calls,
            vec![(thread, vec![Value::integer(7)])]
        );
        assert_eq!(
            runtime.thread_status(thread).expect("thread has status"),
            ThreadStatus::Dead
        );
    }

    #[test]
    fn coroutine_resume_returns_false_and_error_message_on_failure() {
        let mut runtime = TestRuntime::default();
        let thread = runtime.push_thread(ThreadStatus::Runnable);
        runtime.resume_results.insert(0, Err("boom".into()));

        let values = coroutine_resume(&mut runtime, &[thread]).expect("resume should pass");

        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::boolean(false));
        assert_eq!(
            runtime.short_string_bytes(values[1]),
            Some(b"boom".as_slice())
        );
    }

    #[test]
    fn coroutine_yield_delegates_values_to_runtime() {
        let mut runtime = TestRuntime {
            yield_result: Ok(vec![Value::integer(8)]),
            ..TestRuntime::default()
        };

        let values = coroutine_yield(&mut runtime, &[Value::integer(1), Value::boolean(false)])
            .expect("yield should pass");

        assert_eq!(values, vec![Value::integer(8)]);
        assert_eq!(
            runtime.yield_calls,
            vec![vec![Value::integer(1), Value::boolean(false)]]
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
    fn coroutine_running_returns_current_thread_and_main_flag() {
        let mut runtime = TestRuntime::default();
        let thread = runtime.push_thread(ThreadStatus::Running);
        runtime.running = Some((thread, true));

        assert_eq!(
            coroutine_running(&mut runtime, &[]).expect("running should pass"),
            vec![thread, Value::boolean(true)]
        );
    }

    #[test]
    fn coroutine_isyieldable_reports_thread_yieldability() {
        let mut runtime = TestRuntime::default();
        let main = runtime.push_thread(ThreadStatus::Running);
        let runnable = runtime.push_thread(ThreadStatus::Runnable);
        let dead = runtime.push_thread(ThreadStatus::Dead);
        runtime.running = Some((main, true));

        assert_eq!(
            coroutine_isyieldable(&mut runtime, &[]).expect("isyieldable should pass"),
            vec![Value::boolean(false)]
        );
        assert_eq!(
            coroutine_isyieldable(&mut runtime, &[runnable]).expect("isyieldable should pass"),
            vec![Value::boolean(true)]
        );
        assert_eq!(
            coroutine_isyieldable(&mut runtime, &[dead]).expect("isyieldable should pass"),
            vec![Value::boolean(false)]
        );
    }

    #[test]
    fn coroutine_isyieldable_requires_thread_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            coroutine_isyieldable(&mut runtime, &[Value::integer(1)])
                .expect_err("non-thread should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "thread"
            }
        );
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
    fn coroutine_resume_requires_thread_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            coroutine_resume(&mut runtime, &[Value::nil()])
                .expect_err("non-thread should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "thread"
            }
        );
    }

    #[test]
    fn coroutine_close_requires_thread_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            coroutine_close(&mut runtime, &[Value::integer(1)])
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
