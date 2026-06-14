use elara_core::Value;

use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib, native_functions};

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
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "lines")));
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
fn io_lines_reports_unsupported_file_handles() {
    let function = native_functions(StdLib::Io)
        .iter()
        .find(|function| function.descriptor().name() == "lines")
        .expect("io.lines native function should exist")
        .function();
    let mut runtime = TestRuntime::default();
    let filename = runtime.push_string(b"file.txt");
    let format = runtime.push_string(b"*l");

    let result = function(&mut runtime, &[filename, format, Value::integer(8)])
        .expect("io.lines should validate supported arguments");

    assert_eq!(result[0], Value::nil());
    assert_eq!(
        runtime.bytes(result[1]),
        Some(b"file handles are not supported by this runtime".as_slice())
    );
}

#[test]
fn io_lines_validates_arguments() {
    let function = native_functions(StdLib::Io)
        .iter()
        .find(|function| function.descriptor().name() == "lines")
        .expect("io.lines native function should exist")
        .function();
    let mut runtime = TestRuntime::default();

    assert!(function(&mut runtime, &[]).is_ok());
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
        function(&mut runtime, &[Value::nil(), Value::boolean(false)])
            .expect_err("non-format line argument")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "string or integer",
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
