//! Executable I/O library natives.

use elara_core::Value;

use crate::{FunctionSpec, NativeError, NativeFunctionSpec, NativeRuntime, StdLib};

/// Executable `io` library functions currently implemented.
pub const IO_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Io, "type"),
    io_type,
)];

fn io_type(_runtime: &mut dyn NativeRuntime, _args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::nil()])
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{FunctionSpec, NativeError, NativeRuntime, StdLib, native_functions};

    #[derive(Default)]
    struct TestRuntime;

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, NativeError> {
            unreachable!("io.type should not allocate strings without file handles")
        }

        fn short_string_bytes(&self, _value: Value) -> Option<&[u8]> {
            None
        }
    }

    #[test]
    fn io_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = super::IO_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Io, "type")));
    }

    #[test]
    fn io_type_returns_nil_without_file_handles() {
        let function = native_functions(StdLib::Io)
            .iter()
            .find(|function| function.descriptor().name() == "type")
            .expect("io.type native function should exist")
            .function();
        let mut runtime = TestRuntime;

        assert_eq!(
            function(&mut runtime, &[]).expect("io.type should pass"),
            vec![Value::nil()]
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)]).expect("io.type should pass"),
            vec![Value::nil()]
        );
    }
}
