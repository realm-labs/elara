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
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Io, "lines"), io_lines),
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

fn io_lines(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    optional_string_arg(runtime, args, 1)?;
    for (offset, value) in args.iter().copied().enumerate().skip(1) {
        string_or_integer_arg(runtime, value, offset + 1)?;
    }

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
#[path = "io_tests.rs"]
mod tests;
