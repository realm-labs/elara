//! Executable debug library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable `debug` library functions currently implemented.
pub const DEBUG_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "getmetatable"),
        debug_getmetatable,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "setmetatable"),
        debug_setmetatable,
    ),
];

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

fn debug_setmetatable(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = args
        .first()
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !value.is_table() {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "table",
        }
        .into());
    }
    let metatable = args
        .get(1)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    if !metatable.is_nil() && !metatable.is_table() {
        return Err(NativeErrorKind::TypeError {
            index: 2,
            expected: "nil or table",
        }
        .into());
    }
    runtime.table_set_metatable(value, metatable)?;
    Ok(vec![value])
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{NativeErrorKind, NativeRuntime, StdLib, native_functions};

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

    impl NativeRuntime for TestRuntime {
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

        fn table_set_metatable(
            &mut self,
            table: Value,
            metatable: Value,
        ) -> Result<(), crate::NativeError> {
            let index = table.as_table_index().ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "table",
            })? as usize;
            self.metatables[index] = metatable;
            Ok(())
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

    #[test]
    fn debug_setmetatable_sets_raw_table_metatable() {
        let function = function("setmetatable");
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(Value::nil());
        let metatable = runtime.push_table(Value::nil());

        assert_eq!(
            function(&mut runtime, &[table, metatable]).expect("debug.setmetatable should pass"),
            vec![table]
        );
        assert_eq!(runtime.table_metatable(table), Ok(metatable));

        assert_eq!(
            function(&mut runtime, &[table, Value::nil()])
                .expect("debug.setmetatable nil should pass"),
            vec![table]
        );
        assert_eq!(runtime.table_metatable(table), Ok(Value::nil()));
    }

    #[test]
    fn debug_setmetatable_validates_arguments() {
        let function = function("setmetatable");
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(Value::nil());

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing value")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1), table])
                .expect_err("non-table value")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "table",
            }
        );
        assert_eq!(
            function(&mut runtime, &[table])
                .expect_err("missing metatable")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 2 }
        );
        assert_eq!(
            function(&mut runtime, &[table, Value::integer(1)])
                .expect_err("bad metatable")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "nil or table",
            }
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
