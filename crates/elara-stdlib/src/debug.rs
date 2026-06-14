//! Executable debug library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable `debug` library functions currently implemented.
pub const DEBUG_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Debug, "getmetatable"),
    debug_getmetatable,
)];

fn debug_getmetatable(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = args
        .first()
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !value.is_table() {
        return Ok(vec![Value::nil()]);
    }
    Ok(vec![runtime.table_metatable(value)?])
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{NativeErrorKind, StdLib, native_functions};

    #[derive(Default)]
    struct TestRuntime {
        metatables: Vec<Value>,
    }

    impl TestRuntime {
        fn push_table(&mut self, metatable: Value) -> Value {
            let index = u32::try_from(self.metatables.len()).expect("test table index fits in u32");
            self.metatables.push(metatable);
            Value::table_index(index)
        }
    }

    impl crate::NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, crate::NativeError> {
            unreachable!("debug.getmetatable does not intern strings")
        }

        fn short_string_bytes(&self, _value: Value) -> Option<&[u8]> {
            unreachable!("debug.getmetatable does not read strings")
        }

        fn table_metatable(&self, table: Value) -> Result<Value, crate::NativeError> {
            let index = table.as_table_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "table",
            })? as usize;
            Ok(*self.metatables.get(index).unwrap_or(&Value::nil()))
        }
    }

    #[test]
    fn debug_getmetatable_returns_raw_table_metatable() {
        let function = function("getmetatable");
        let mut runtime = TestRuntime::default();
        let metatable = runtime.push_table(Value::nil());
        let table = runtime.push_table(metatable);

        assert_eq!(
            function(&mut runtime, &[table]).expect("debug.getmetatable should pass"),
            vec![metatable]
        );
    }

    #[test]
    fn debug_getmetatable_returns_nil_without_table_metatable() {
        let function = function("getmetatable");
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(Value::nil());

        assert_eq!(
            function(&mut runtime, &[table]).expect("debug.getmetatable should pass"),
            vec![Value::nil()]
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)]).expect("non-table should pass"),
            vec![Value::nil()]
        );
    }

    #[test]
    fn debug_getmetatable_validates_argument() {
        let function = function("getmetatable");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing argument")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
    }

    fn function(name: &str) -> crate::NativeStdFunction {
        native_functions(StdLib::Debug)
            .iter()
            .find(|function| function.descriptor().name() == name)
            .expect("debug native function should exist")
            .function()
    }
}
