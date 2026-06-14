//! Executable debug library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable `debug` library functions currently implemented.
pub const DEBUG_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Debug, "gethook"), debug_gethook),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "getmetatable"),
        debug_getmetatable,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "setmetatable"),
        debug_setmetatable,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "getregistry"),
        debug_getregistry,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "traceback"),
        debug_traceback,
    ),
];

fn debug_gethook(
    _runtime: &mut dyn NativeRuntime,
    _args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::nil()])
}

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

fn debug_getregistry(
    runtime: &mut dyn NativeRuntime,
    _args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    Ok(vec![runtime.debug_registry()?])
}

fn debug_traceback(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let has_thread_arg = args.first().is_some_and(|value| value.is_thread());
    let message_index = usize::from(has_thread_arg);
    let message = args.get(message_index).copied().unwrap_or_else(Value::nil);
    let Some(message) = traceback_message(runtime, message)? else {
        return Ok(vec![message]);
    };
    let level_index = message_index + 1;
    let default_level = if has_thread_arg { 0 } else { 1 };
    let level = optional_integer_arg(args, level_index, default_level)?;
    Ok(vec![runtime.debug_traceback(message.as_deref(), level)?])
}

fn traceback_message(
    runtime: &dyn NativeRuntime,
    value: Value,
) -> Result<Option<Option<Vec<u8>>>, NativeError> {
    if value.is_nil() {
        Ok(Some(None))
    } else if let Some(bytes) = runtime.string_bytes(value) {
        Ok(Some(Some(bytes.to_vec())))
    } else if let Some(value) = value.as_integer() {
        Ok(Some(Some(value.to_string().into_bytes())))
    } else if let Some(value) = value.as_float() {
        Ok(Some(Some(value.to_string().into_bytes())))
    } else if value.is_string() {
        Err(NativeErrorKind::RuntimeError {
            message: "debug.traceback string value is not owned by this runtime".into(),
        }
        .into())
    } else {
        Ok(None)
    }
}

fn optional_integer_arg(args: &[Value], index: usize, default: i64) -> Result<i64, NativeError> {
    match args.get(index) {
        None => Ok(default),
        Some(value) if value.is_nil() => Ok(default),
        Some(value) => value.as_integer().ok_or(
            NativeErrorKind::TypeError {
                index: index + 1,
                expected: "integer",
            }
            .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{NativeErrorKind, NativeRuntime, StdLib, native_functions};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        metatables: Vec<Value>,
        registry: Option<Value>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::closure_index(index)
        }

        fn push_table(&mut self, metatable: Value) -> Value {
            let index = u32::try_from(self.metatables.len()).expect("test table index fits in u32");
            self.metatables.push(metatable);
            Value::table_index(index)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, crate::NativeError> {
            unreachable!("debug natives use intern_string instead")
        }

        fn intern_string(&mut self, bytes: &[u8]) -> Result<Value, crate::NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
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

        fn debug_registry(&mut self) -> Result<Value, crate::NativeError> {
            if let Some(registry) = self.registry {
                return Ok(registry);
            }
            let registry = self.push_table(Value::nil());
            self.registry = Some(registry);
            Ok(registry)
        }
    }

    #[test]
    fn debug_gethook_returns_nil_without_installed_hook() {
        let function = function("gethook");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[]).expect("debug.gethook should pass"),
            vec![Value::nil()]
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)]).expect("non-thread arg should pass"),
            vec![Value::nil()]
        );
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

    #[test]
    fn debug_getregistry_returns_stable_registry_table() {
        let function = function("getregistry");
        let mut runtime = TestRuntime::default();

        let first = function(&mut runtime, &[]).expect("debug.getregistry should pass");
        let second = function(&mut runtime, &[]).expect("debug.getregistry should pass");

        assert_eq!(first.len(), 1);
        assert!(first[0].is_table());
        assert_eq!(first, second);
    }

    #[test]
    fn debug_traceback_returns_standard_header_without_message() {
        let function = function("traceback");
        let mut runtime = TestRuntime::default();

        let values = function(&mut runtime, &[]).expect("debug.traceback should pass");

        assert_eq!(values.len(), 1);
        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"stack traceback:".as_slice())
        );
    }

    #[test]
    fn debug_traceback_prefixes_string_and_number_messages() {
        let function = function("traceback");
        let mut runtime = TestRuntime::default();
        let message = runtime.push_string(b"boom");

        let values = function(&mut runtime, &[message]).expect("string message should pass");
        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"boom\nstack traceback:".as_slice())
        );

        let values = function(&mut runtime, &[Value::integer(12)]).expect("number should pass");
        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"12\nstack traceback:".as_slice())
        );
    }

    #[test]
    fn debug_traceback_returns_non_string_message_unchanged() {
        let function = function("traceback");
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(Value::nil());

        assert_eq!(
            function(&mut runtime, &[Value::boolean(true)]).expect("boolean message should pass"),
            vec![Value::boolean(true)]
        );
        assert_eq!(
            function(&mut runtime, &[table]).expect("table message should pass"),
            vec![table]
        );
    }

    #[test]
    fn debug_traceback_validates_level() {
        let function = function("traceback");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[Value::nil(), Value::boolean(false)])
                .expect_err("level should be an integer")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "integer",
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
