//! Executable debug library natives.

use elara_core::Value;

use crate::{
    DebugInfoTarget, FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime,
    StdLib,
};

/// Executable `debug` library functions currently implemented.
pub const DEBUG_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Debug, "gethook"), debug_gethook),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Debug, "getinfo"), debug_getinfo),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "getuservalue"),
        debug_getuservalue,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "getupvalue"),
        debug_getupvalue,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "getmetatable"),
        debug_getmetatable,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "setmetatable"),
        debug_setmetatable,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "setupvalue"),
        debug_setupvalue,
    ),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Debug, "sethook"), debug_sethook),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Debug, "setuservalue"),
        debug_setuservalue,
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

fn debug_getinfo(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let has_thread_arg = args.first().is_some_and(|value| value.is_thread());
    let target_index = usize::from(has_thread_arg);
    let target = args
        .get(target_index)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument {
            index: target_index + 1,
        })?;
    let target = if let Some(level) = target.as_integer() {
        DebugInfoTarget::Level(level)
    } else if target.is_closure() {
        DebugInfoTarget::Function(target)
    } else {
        return Err(NativeErrorKind::TypeError {
            index: target_index + 1,
            expected: "function or integer",
        }
        .into());
    };
    let options = optional_string_arg(runtime, args, target_index + 1)?.map(<[u8]>::to_vec);
    Ok(vec![runtime.debug_getinfo(target, options.as_deref())?])
}

fn debug_getuservalue(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    optional_integer_arg(args, 1, 1)?;
    Ok(vec![Value::nil()])
}

fn debug_getupvalue(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let function = args
        .first()
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !function.is_closure() {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "function",
        }
        .into());
    }
    let index = integer_arg(args, 1)?;
    Ok(
        if let Some((name, value)) = runtime.debug_getupvalue(function, index)? {
            vec![name, value]
        } else {
            vec![Value::nil()]
        },
    )
}

fn debug_setupvalue(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let function = args
        .first()
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !function.is_closure() {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "function",
        }
        .into());
    }
    let index = integer_arg(args, 1)?;
    let value = args
        .get(2)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 3 })?;
    Ok(
        if let Some(name) = runtime.debug_setupvalue(function, index, value)? {
            vec![name]
        } else {
            vec![Value::nil()]
        },
    )
}

fn debug_sethook(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let has_thread_arg = args.first().is_some_and(|value| value.is_thread());
    let hook_index = usize::from(has_thread_arg);
    let hook = args.get(hook_index).copied().unwrap_or_else(Value::nil);
    if hook.is_nil() {
        return Ok(Vec::new());
    }
    if !hook.is_closure() {
        return Err(NativeErrorKind::TypeError {
            index: hook_index + 1,
            expected: "function",
        }
        .into());
    }
    let mask = args
        .get(hook_index + 1)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument {
            index: hook_index + 2,
        })?;
    if runtime.string_bytes(mask).is_none() {
        return Err(NativeErrorKind::TypeError {
            index: hook_index + 2,
            expected: "string",
        }
        .into());
    }
    if let Some(count) = args.get(hook_index + 2).filter(|value| !value.is_nil()) {
        count.as_integer().ok_or(NativeErrorKind::TypeError {
            index: hook_index + 3,
            expected: "integer",
        })?;
    }
    Err(NativeErrorKind::RuntimeError {
        message: "debug hook callbacks are not supported yet".into(),
    }
    .into())
}

fn debug_setuservalue(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    args.first()
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    Err(NativeErrorKind::TypeError {
        index: 1,
        expected: "userdata",
    }
    .into())
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

fn integer_arg(args: &[Value], index: usize) -> Result<i64, NativeError> {
    args.get(index)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: index + 1 })?
        .as_integer()
        .ok_or(NativeErrorKind::TypeError {
            index: index + 1,
            expected: "integer",
        })
        .map_err(Into::into)
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

fn optional_string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Option<&'a [u8]>, NativeError> {
    let Some(value) = args.get(index).copied() else {
        return Ok(None);
    };
    if value.is_nil() {
        return Ok(None);
    }
    runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index: index + 1,
            expected: "string",
        })
        .map(Some)
        .map_err(Into::into)
}

#[cfg(test)]
#[path = "debug_upvalue_tests.rs"]
mod upvalue_tests;

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{DebugInfoTarget, NativeErrorKind, NativeRuntime, StdLib, native_functions};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        metatables: Vec<Value>,
        registry: Option<Value>,
        debug_info: Option<Value>,
        debug_info_request: Option<(DebugInfoTarget, Option<Vec<u8>>)>,
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

        fn debug_getinfo(
            &mut self,
            target: DebugInfoTarget,
            options: Option<&[u8]>,
        ) -> Result<Value, crate::NativeError> {
            self.debug_info_request = Some((target, options.map(<[u8]>::to_vec)));
            Ok(self.debug_info.unwrap_or_else(Value::nil))
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
    fn debug_sethook_clears_hooks_without_results() {
        let function = function("sethook");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[]).expect("debug.sethook should clear hooks"),
            Vec::<Value>::new()
        );
        assert_eq!(
            function(&mut runtime, &[Value::nil()]).expect("debug.sethook(nil) should clear hooks"),
            Vec::<Value>::new()
        );
    }

    #[test]
    fn debug_sethook_rejects_hook_installation_until_supported() {
        let function = function("sethook");
        let mut runtime = TestRuntime::default();
        let hook = Value::native_function_index(1);
        let mask = runtime.push_string(b"c");

        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-function hook should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "function",
            }
        );
        assert_eq!(
            function(&mut runtime, &[hook, mask])
                .expect_err("hook callbacks are not supported")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "debug hook callbacks are not supported yet".into(),
            }
        );
    }

    #[test]
    fn debug_getinfo_forwards_level_queries() {
        let function = function("getinfo");
        let mut runtime = TestRuntime {
            debug_info: Some(Value::table_index(7)),
            ..TestRuntime::default()
        };

        assert_eq!(
            function(&mut runtime, &[Value::integer(2)]).expect("debug.getinfo should pass"),
            vec![Value::table_index(7)]
        );
        assert_eq!(
            runtime.debug_info_request,
            Some((DebugInfoTarget::Level(2), None))
        );
    }

    #[test]
    fn debug_getinfo_forwards_function_queries_and_options() {
        let function = function("getinfo");
        let mut runtime = TestRuntime {
            debug_info: Some(Value::table_index(9)),
            ..TestRuntime::default()
        };
        let target = Value::native_function_index(3);
        let options = runtime.push_string(b"nSl");

        assert_eq!(
            function(&mut runtime, &[target, options]).expect("debug.getinfo should pass"),
            vec![Value::table_index(9)]
        );
        assert_eq!(
            runtime.debug_info_request,
            Some((DebugInfoTarget::Function(target), Some(b"nSl".to_vec())))
        );
    }

    #[test]
    fn debug_getinfo_validates_arguments() {
        let function = function("getinfo");
        let mut runtime = TestRuntime::default();
        let target = Value::native_function_index(3);

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing target")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::boolean(false)])
                .expect_err("bad target")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "function or integer",
            }
        );
        assert_eq!(
            function(&mut runtime, &[target, Value::boolean(false)])
                .expect_err("bad options")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "string",
            }
        );
    }

    #[test]
    fn debug_getuservalue_returns_nil_for_non_userdata() {
        let function = function("getuservalue");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[]).expect("missing receiver should pass"),
            vec![Value::nil()]
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)]).expect("non-userdata should pass"),
            vec![Value::nil()]
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1), Value::boolean(false)])
                .expect_err("user value index should be an integer")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "integer",
            }
        );
    }

    #[test]
    fn debug_setuservalue_rejects_current_non_userdata_values() {
        let function = function("setuservalue");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing receiver should fail")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1), Value::integer(2)])
                .expect_err("non-userdata should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "userdata",
            }
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
