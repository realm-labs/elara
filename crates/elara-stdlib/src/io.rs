//! Executable I/O library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

const FILE_HANDLES_UNSUPPORTED: &[u8] = b"file handles are not supported by this runtime";

/// Executable `io` library functions currently implemented.
pub const IO_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "close"), io_close),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "flush"), io_flush),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "input"), io_input),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "open"), io_open),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "output"), io_output),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "popen"), io_popen),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "read"), io_read),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "tmpfile"), io_tmpfile),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "type"), io_type),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "write"), io_write),
];

fn io_close(runtime: &mut dyn NativeRuntime, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
    unsupported_file_result(runtime)
}

fn io_flush(runtime: &mut dyn NativeRuntime, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
    unsupported_file_result(runtime)
}

fn io_input(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    optional_string_or_file_arg(runtime, args, 1)?;

    unsupported_file_result(runtime)
}

fn io_open(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    string_arg(runtime, args, 1)?;
    optional_string_arg(runtime, args, 2)?;

    unsupported_file_result(runtime)
}

fn io_output(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    optional_string_or_file_arg(runtime, args, 1)?;

    unsupported_file_result(runtime)
}

fn io_popen(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    string_arg(runtime, args, 1)?;
    optional_string_arg(runtime, args, 2)?;

    unsupported_file_result(runtime)
}

fn io_read(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    for (offset, value) in args.iter().copied().enumerate() {
        string_or_integer_arg(runtime, value, offset + 1)?;
    }

    unsupported_file_result(runtime)
}

fn io_tmpfile(runtime: &mut dyn NativeRuntime, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
    unsupported_file_result(runtime)
}

fn io_type(_runtime: &mut dyn NativeRuntime, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::nil()])
}

fn io_write(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    for (offset, value) in args.iter().copied().enumerate() {
        string_or_number_arg(runtime, value, offset + 1)?;
    }

    unsupported_file_result(runtime)
}

fn unsupported_file_result(runtime: &mut dyn NativeRuntime) -> Result<Vec<Value>, NativeError> {
    Ok(vec![
        Value::nil(),
        runtime.intern_string(FILE_HANDLES_UNSUPPORTED)?,
    ])
}

fn string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<&'a [u8], NativeError> {
    let value = args
        .get(index - 1)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string",
        })
        .map_err(Into::into)
}

fn optional_string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Option<&'a [u8]>, NativeError> {
    let Some(value) = args.get(index - 1).copied() else {
        return Ok(None);
    };
    if value.is_nil() {
        return Ok(None);
    }
    runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string",
        })
        .map(Some)
        .map_err(Into::into)
}

fn string_or_integer_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<(), NativeError> {
    if value.as_integer().is_some() || runtime.string_bytes(value).is_some() {
        return Ok(());
    }
    Err(NativeErrorKind::TypeError {
        index,
        expected: "string or integer",
    }
    .into())
}

fn string_or_number_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<(), NativeError> {
    if value.is_number() || runtime.string_bytes(value).is_some() {
        return Ok(());
    }
    Err(NativeErrorKind::TypeError {
        index,
        expected: "string or number",
    }
    .into())
}

fn optional_string_or_file_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Option<&'a [u8]>, NativeError> {
    let Some(value) = args.get(index - 1).copied() else {
        return Ok(None);
    };
    if value.is_nil() {
        return Ok(None);
    }
    runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string or file",
        })
        .map(Some)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{
        FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib, native_functions,
    };

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

        fn bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            self.bytes(value)
        }
    }

    #[test]
    fn io_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = super::IO_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "close")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "flush")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "input")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "type")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "open")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "output")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "popen")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "read")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "tmpfile")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "write")));
    }

    #[test]
    fn io_close_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "close")
            .expect("io.close native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        let result = function(&mut runtime, &[]).expect("io.close should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_flush_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "flush")
            .expect("io.flush native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        let result = function(&mut runtime, &[]).expect("io.flush should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_input_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "input")
            .expect("io.input native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let filename = runtime.push_string(b"file.txt");

        let result = function(&mut runtime, &[filename]).expect("io.input should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_input_validates_arguments() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "input")
            .expect("io.input native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        assert!(function(&mut runtime, &[]).is_ok());
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string input target")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string or file",
            }
        );
    }

    #[test]
    fn io_open_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "open")
            .expect("io.open native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let filename = runtime.push_string(b"file.txt");
        let mode = runtime.push_string(b"r");

        let result = function(&mut runtime, &[filename, mode]).expect("io.open should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_open_validates_arguments() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "open")
            .expect("io.open native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let filename = runtime.push_string(b"file.txt");

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing filename")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string filename")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
        assert_eq!(
            function(&mut runtime, &[filename, Value::integer(1)])
                .expect_err("non-string mode")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "string",
            }
        );
    }

    #[test]
    fn io_output_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "output")
            .expect("io.output native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let filename = runtime.push_string(b"file.txt");

        let result = function(&mut runtime, &[filename]).expect("io.output should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_output_validates_arguments() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "output")
            .expect("io.output native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        assert!(function(&mut runtime, &[]).is_ok());
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string output target")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string or file",
            }
        );
    }

    #[test]
    fn io_tmpfile_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "tmpfile")
            .expect("io.tmpfile native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        let result = function(&mut runtime, &[]).expect("io.tmpfile should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_popen_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "popen")
            .expect("io.popen native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let command = runtime.push_string(b"echo hi");
        let mode = runtime.push_string(b"r");

        let result = function(&mut runtime, &[command, mode]).expect("io.popen should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_popen_validates_arguments() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "popen")
            .expect("io.popen native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let command = runtime.push_string(b"echo hi");

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing command")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string command")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
        assert_eq!(
            function(&mut runtime, &[command, Value::integer(1)])
                .expect_err("non-string mode")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "string",
            }
        );
    }

    #[test]
    fn io_read_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "read")
            .expect("io.read native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let format = runtime.push_string(b"*l");

        let result = function(&mut runtime, &[format, Value::integer(8)])
            .expect("io.read should validate supported format arguments");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_read_validates_arguments() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "read")
            .expect("io.read native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        assert!(function(&mut runtime, &[]).is_ok());
        assert_eq!(
            function(&mut runtime, &[Value::boolean(false)])
                .expect_err("non-format read argument")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string or integer",
            }
        );
    }

    #[test]
    fn io_type_returns_nil_without_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "type")
            .expect("io.type native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[]).expect("io.type should pass"),
            vec![Value::nil()]
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)]).expect("io.type should pass"),
            vec![Value::nil()]
        );
    }

    #[test]
    fn io_write_reports_unsupported_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "write")
            .expect("io.write native function should exist")
            .function();
        let mut runtime = TestRuntime::default();
        let text = runtime.push_string(b"hello");

        let result = function(&mut runtime, &[text, Value::integer(7), Value::float(1.5)])
            .expect("io.write should validate supported values");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"file handles are not supported by this runtime".as_slice())
        );
    }

    #[test]
    fn io_write_validates_arguments() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "write")
            .expect("io.write native function should exist")
            .function();
        let mut runtime = TestRuntime::default();

        assert!(function(&mut runtime, &[]).is_ok());
        assert_eq!(
            function(&mut runtime, &[Value::boolean(false)])
                .expect_err("non-writable value")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string or number",
            }
        );
    }
}
