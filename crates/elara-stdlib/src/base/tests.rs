use elara_core::Value;

use super::{
    BASE_NATIVE_FUNCTIONS, base_assert, base_collectgarbage, base_dofile, base_error,
    base_getmetatable, base_ipairs, base_ipairs_aux, base_load, base_loadfile, base_next,
    base_pairs, base_pcall, base_print, base_rawequal, base_rawget, base_rawlen, base_rawset,
    base_select, base_setmetatable, base_tonumber, base_tostring, base_type, base_warn,
    base_xpcall,
};
use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

struct TestRuntime {
    strings: Vec<Box<[u8]>>,
    tables: Vec<Vec<(Value, Value)>>,
    metatables: Vec<Option<Value>>,
    output: Vec<u8>,
    warnings: Vec<u8>,
    warnings_enabled: bool,
    base_next: Value,
    base_ipairs_aux: Value,
    protected_results: Vec<Result<Vec<Value>, Box<str>>>,
    protected_calls: Vec<(Value, Vec<Value>)>,
}

impl Default for TestRuntime {
    fn default() -> Self {
        Self {
            strings: Vec::new(),
            tables: Vec::new(),
            metatables: Vec::new(),
            output: Vec::new(),
            warnings: Vec::new(),
            warnings_enabled: false,
            base_next: Value::nil(),
            base_ipairs_aux: Value::nil(),
            protected_results: Vec::new(),
            protected_calls: Vec::new(),
        }
    }
}

impl TestRuntime {
    fn push_string(&mut self, bytes: &[u8]) -> Value {
        if let Some(index) = self
            .strings
            .iter()
            .position(|string| string.as_ref() == bytes)
        {
            let index = u32::try_from(index).expect("test string index fits in u32");
            return Value::closure_index(index);
        }
        let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
        self.strings.push(bytes.into());
        Value::closure_index(index)
    }

    fn push_table(&mut self, entries: Vec<(Value, Value)>) -> Value {
        let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
        self.tables.push(entries);
        self.metatables.push(None);
        Value::table_index(index)
    }
}

impl NativeRuntime for TestRuntime {
    fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
        Ok(self.push_string(bytes))
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        let index = value.as_closure_index()? as usize;
        self.strings.get(index).map(Box::as_ref)
    }

    fn table_array_len(&self, table: Value) -> Result<i64, NativeError> {
        let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
        let table = self.tables.get(table_index).ok_or_else(non_table_error)?;
        Ok(table
            .iter()
            .filter_map(|(key, value)| {
                (!value.is_nil())
                    .then(|| key.as_integer())
                    .flatten()
                    .filter(|key| *key > 0)
            })
            .max()
            .unwrap_or(0))
    }

    fn table_get(&self, table: Value, key: Value) -> Result<Value, NativeError> {
        let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
        let table = self.tables.get(table_index).ok_or_else(non_table_error)?;
        Ok(table
            .iter()
            .rev()
            .find_map(|(entry_key, entry_value)| {
                (*entry_key == key && !entry_value.is_nil()).then_some(*entry_value)
            })
            .unwrap_or_else(Value::nil))
    }

    fn table_get_integer(&self, table: Value, index: i64) -> Result<Value, NativeError> {
        self.table_get(table, Value::integer(index))
    }

    fn table_next(&self, table: Value, key: Value) -> Result<Option<(Value, Value)>, NativeError> {
        let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
        let table = self.tables.get(table_index).ok_or_else(non_table_error)?;
        let start = if key.is_nil() {
            0
        } else if let Some(position) = table.iter().position(|(entry_key, _)| *entry_key == key) {
            position + 1
        } else {
            table.len()
        };
        Ok(table
            .iter()
            .copied()
            .skip(start)
            .find(|(_, value)| !value.is_nil()))
    }

    fn table_set(&mut self, table: Value, key: Value, value: Value) -> Result<(), NativeError> {
        let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
        let table = self
            .tables
            .get_mut(table_index)
            .ok_or_else(non_table_error)?;
        if key.is_nil() {
            return Err(NativeErrorKind::RuntimeError {
                message: "table index is nil or NaN".into(),
            }
            .into());
        }
        table.push((key, value));
        Ok(())
    }

    fn table_metatable(&self, table: Value) -> Result<Value, NativeError> {
        let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
        self.tables.get(table_index).ok_or_else(non_table_error)?;
        Ok(self
            .metatables
            .get(table_index)
            .copied()
            .flatten()
            .unwrap_or_else(Value::nil))
    }

    fn table_set_metatable(&mut self, table: Value, metatable: Value) -> Result<(), NativeError> {
        let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
        self.tables.get(table_index).ok_or_else(non_table_error)?;
        if !metatable.is_nil() {
            let metatable_index = metatable.as_table_index().ok_or_else(non_table_error)? as usize;
            self.tables
                .get(metatable_index)
                .ok_or_else(non_table_error)?;
        }
        self.metatables[table_index] = (!metatable.is_nil()).then_some(metatable);
        Ok(())
    }

    fn write_output(&mut self, bytes: &[u8]) -> Result<(), NativeError> {
        self.output.extend_from_slice(bytes);
        Ok(())
    }

    fn warnings_enabled(&self) -> bool {
        self.warnings_enabled
    }

    fn set_warnings_enabled(&mut self, enabled: bool) -> Result<(), NativeError> {
        self.warnings_enabled = enabled;
        Ok(())
    }

    fn write_warning(&mut self, bytes: &[u8]) -> Result<(), NativeError> {
        self.warnings.extend_from_slice(bytes);
        Ok(())
    }

    fn protected_call(
        &mut self,
        function: Value,
        args: &[Value],
    ) -> Result<Result<Vec<Value>, Box<str>>, NativeError> {
        self.protected_calls.push((function, args.to_vec()));
        if self.protected_results.is_empty() {
            return Ok(Err("missing protected result".into()));
        }
        Ok(self.protected_results.remove(0))
    }

    fn native_function(&self, library: StdLib, name: &str) -> Result<Value, NativeError> {
        match (library, name) {
            (StdLib::Base, "next") => Ok(self.base_next),
            (StdLib::Base, "__ipairs_aux") => Ok(self.base_ipairs_aux),
            _ => Err(NativeErrorKind::RuntimeError {
                message: "unknown native helper".into(),
            }
            .into()),
        }
    }
}

fn non_table_error() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "attempt to index a non-table value".into(),
    }
    .into()
}

fn call(function: crate::NativeStdFunction, args: &[Value]) -> Vec<Value> {
    function(&mut TestRuntime::default(), args).expect("native should pass")
}

fn call_with_runtime(
    runtime: &mut TestRuntime,
    function: crate::NativeStdFunction,
    args: &[Value],
) -> Vec<Value> {
    function(runtime, args).expect("native should pass")
}

#[test]
fn base_native_specs_cover_executable_subset() {
    let descriptors: Vec<_> = BASE_NATIVE_FUNCTIONS
        .iter()
        .map(|function| function.descriptor())
        .collect();

    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "assert")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "collectgarbage")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "dofile")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "error")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "getmetatable")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "ipairs")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "load")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "loadfile")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "next")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "pairs")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "pcall")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "print")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawequal")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawget")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawlen")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawset")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "select")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "setmetatable")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "tonumber")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "tostring")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "type")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "warn")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "xpcall")));
}

#[test]
fn base_dynamic_loading_functions_report_unsupported_results() {
    let mut runtime = TestRuntime::default();
    let chunk = runtime.push_string(b"return 42");
    let filename = runtime.push_string(b"chunk.lua");

    let values = call_with_runtime(&mut runtime, base_load, &[chunk]);
    assert_eq!(values[0], Value::nil());
    assert_eq!(
        runtime.short_string_bytes(values[1]),
        Some(b"dynamic Lua loading is not supported".as_slice())
    );

    let values = call_with_runtime(&mut runtime, base_loadfile, &[filename]);
    assert_eq!(values[0], Value::nil());
    assert_eq!(
        runtime.short_string_bytes(values[1]),
        Some(b"dynamic Lua loading is not supported".as_slice())
    );

    let error = base_dofile(&mut runtime, &[filename]).expect_err("dofile should fail");
    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "dynamic Lua loading is not supported");
}

#[test]
fn base_load_validates_chunk_and_mode_arguments() {
    assert_eq!(
        base_load(&mut TestRuntime::default(), &[])
            .expect_err("chunk is required")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        base_load(&mut TestRuntime::default(), &[Value::integer(1)])
            .expect_err("chunk should be string or function")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "string or function",
        }
    );

    let mut runtime = TestRuntime::default();
    let chunk = runtime.push_string(b"return 42");
    assert_eq!(
        base_load(
            &mut runtime,
            &[chunk, Value::nil(), Value::boolean(false)]
        )
        .expect_err("mode should be string")
        .kind(),
        &NativeErrorKind::TypeError {
            index: 3,
            expected: "string",
        }
    );
    let binary_mode = runtime.push_string(b"B");
    let error = base_load(&mut runtime, &[chunk, Value::nil(), binary_mode])
        .expect_err("binary mode should fail");
    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "invalid mode");
}

#[test]
fn base_collectgarbage_reports_unsupported_gc_control() {
    let mut runtime = TestRuntime::default();
    let option = runtime.push_string(b"collect");

    let error =
        base_collectgarbage(&mut runtime, &[option]).expect_err("collectgarbage should fail");

    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "collectgarbage is not supported");
}

#[test]
fn base_warn_validates_strings_and_returns_no_values() {
    let mut runtime = TestRuntime::default();
    let first = runtime.push_string(b"first");
    let second = runtime.push_string(b"second");

    assert_eq!(
        base_warn(&mut runtime, &[first, second]).expect("warn should pass"),
        Vec::<Value>::new()
    );
    assert_eq!(
        base_warn(&mut runtime, &[])
            .expect_err("warn needs an argument")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        base_warn(&mut runtime, &[first, Value::integer(1)])
            .expect_err("warn arguments must be strings")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "string",
        }
    );
}

#[test]
fn base_warn_emits_enabled_warning_and_honors_control_messages() {
    let mut runtime = TestRuntime::default();
    let on = runtime.push_string(b"@on");
    let off = runtime.push_string(b"@off");
    let unknown = runtime.push_string(b"@unknown");
    let first = runtime.push_string(b"first");
    let second = runtime.push_string(b"second");

    base_warn(&mut runtime, &[first]).expect("warning should be ignored while off");
    assert!(runtime.warnings.is_empty());

    base_warn(&mut runtime, &[on]).expect("warn @on should pass");
    assert!(runtime.warnings_enabled);
    base_warn(&mut runtime, &[first, second]).expect("warning should emit");
    assert_eq!(runtime.warnings, b"Lua warning: firstsecond\n");

    base_warn(&mut runtime, &[unknown]).expect("unknown control should be ignored");
    assert_eq!(runtime.warnings, b"Lua warning: firstsecond\n");
    base_warn(&mut runtime, &[off]).expect("warn @off should pass");
    assert!(!runtime.warnings_enabled);
    base_warn(&mut runtime, &[first]).expect("warning should be ignored again");
    assert_eq!(runtime.warnings, b"Lua warning: firstsecond\n");
}

#[test]
fn base_pcall_returns_true_and_values_for_successful_call() {
    let mut runtime = TestRuntime {
        protected_results: vec![Ok(vec![Value::integer(42)])],
        ..TestRuntime::default()
    };

    assert_eq!(
        call_with_runtime(
            &mut runtime,
            base_pcall,
            &[Value::native_function_index(3), Value::integer(41)]
        ),
        vec![Value::boolean(true), Value::integer(42)]
    );
    assert_eq!(
        runtime.protected_calls,
        vec![(Value::native_function_index(3), vec![Value::integer(41)])]
    );
}

#[test]
fn base_pcall_returns_false_and_error_message_for_caught_error() {
    let mut runtime = TestRuntime {
        protected_results: vec![Err("boom".into())],
        ..TestRuntime::default()
    };

    let values = call_with_runtime(&mut runtime, base_pcall, &[Value::native_function_index(3)]);

    assert_eq!(values[0], Value::boolean(false));
    assert_eq!(
        runtime.short_string_bytes(values[1]),
        Some(b"boom".as_slice())
    );
}

#[test]
fn base_xpcall_returns_true_and_values_for_successful_call() {
    let mut runtime = TestRuntime {
        protected_results: vec![Ok(vec![Value::integer(42)])],
        ..TestRuntime::default()
    };

    assert_eq!(
        call_with_runtime(
            &mut runtime,
            base_xpcall,
            &[
                Value::native_function_index(3),
                Value::native_function_index(4),
                Value::integer(41),
            ]
        ),
        vec![Value::boolean(true), Value::integer(42)]
    );
    assert_eq!(
        runtime.protected_calls,
        vec![(Value::native_function_index(3), vec![Value::integer(41)])]
    );
}

#[test]
fn base_xpcall_calls_handler_for_caught_error() {
    let mut runtime = TestRuntime {
        protected_results: vec![Err("boom".into()), Ok(vec![Value::integer(99)])],
        ..TestRuntime::default()
    };

    let values = call_with_runtime(
        &mut runtime,
        base_xpcall,
        &[
            Value::native_function_index(3),
            Value::native_function_index(4),
        ],
    );

    assert_eq!(values, vec![Value::boolean(false), Value::integer(99)]);
    assert_eq!(runtime.protected_calls.len(), 2);
    assert_eq!(
        runtime.protected_calls[0],
        (Value::native_function_index(3), vec![])
    );
    assert_eq!(
        runtime.protected_calls[1].0,
        Value::native_function_index(4)
    );
    assert_eq!(
        runtime.short_string_bytes(runtime.protected_calls[1].1[0]),
        Some(b"boom".as_slice())
    );
}

#[test]
fn base_ipairs_returns_hidden_iterator_state_and_zero_control() {
    let mut runtime = TestRuntime {
        base_ipairs_aux: Value::native_function_index(11),
        ..TestRuntime::default()
    };
    let table = runtime.push_table(Vec::new());

    assert_eq!(
        call_with_runtime(&mut runtime, base_ipairs, &[table]),
        vec![Value::native_function_index(11), table, Value::integer(0)]
    );
}

#[test]
fn base_ipairs_aux_iterates_integer_keys_until_nil() {
    let mut runtime = TestRuntime::default();
    let table = runtime.push_table(vec![
        (Value::integer(1), Value::integer(10)),
        (Value::integer(2), Value::integer(20)),
    ]);

    assert_eq!(
        call_with_runtime(&mut runtime, base_ipairs_aux, &[table, Value::integer(0)]),
        vec![Value::integer(1), Value::integer(10)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_ipairs_aux, &[table, Value::integer(1)]),
        vec![Value::integer(2), Value::integer(20)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_ipairs_aux, &[table, Value::integer(2)]),
        vec![Value::nil()]
    );
}

#[test]
fn base_assert_returns_all_arguments_when_truthy() {
    assert_eq!(
        call(
            base_assert,
            &[Value::boolean(true), Value::integer(7), Value::nil()]
        ),
        vec![Value::boolean(true), Value::integer(7), Value::nil()]
    );
}

#[test]
fn base_assert_errors_when_false_or_nil() {
    let false_error = base_assert(&mut TestRuntime::default(), &[Value::boolean(false)])
        .expect_err("false assert should fail");
    assert_eq!(false_error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(false_error.message(), "assertion failed!");

    let nil_error = base_assert(&mut TestRuntime::default(), &[Value::nil()])
        .expect_err("nil assert should fail");
    assert_eq!(nil_error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(nil_error.message(), "assertion failed!");
}

#[test]
fn base_assert_uses_custom_or_default_message() {
    let mut runtime = TestRuntime::default();
    let message = runtime.push_string(b"bad");

    let custom_error = base_assert(&mut runtime, &[Value::boolean(false), message])
        .expect_err("custom assert should fail");
    assert_eq!(custom_error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(custom_error.message(), "bad");

    let nil_message_error = base_assert(&mut runtime, &[Value::boolean(false), Value::nil()])
        .expect_err("nil-message assert should fail");
    assert_eq!(nil_message_error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(nil_message_error.message(), "<no error object>");
}

#[test]
fn base_error_raises_lua_error_with_string_message() {
    let mut runtime = TestRuntime::default();
    let message = runtime.push_string(b"boom");

    let error = base_error(&mut runtime, &[message]).expect_err("error should raise");
    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "boom");
}

#[test]
fn base_print_writes_tab_separated_values_and_newline() {
    let mut runtime = TestRuntime::default();
    let string = runtime.push_string(b"hello");

    assert_eq!(
        base_print(
            &mut runtime,
            &[
                string,
                Value::integer(7),
                Value::nil(),
                Value::boolean(true)
            ]
        )
        .expect("print should pass"),
        Vec::<Value>::new()
    );
    assert_eq!(runtime.output, b"hello\t7\tnil\ttrue\n");
}

#[test]
fn base_print_without_arguments_writes_newline() {
    let mut runtime = TestRuntime::default();

    assert_eq!(
        base_print(&mut runtime, &[]).expect("print should pass"),
        Vec::<Value>::new()
    );
    assert_eq!(runtime.output, b"\n");
}

#[test]
fn base_error_accepts_absent_or_nil_message() {
    let error = base_error(&mut TestRuntime::default(), &[]).expect_err("error should raise");
    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "<no error object>");

    let error =
        base_error(&mut TestRuntime::default(), &[Value::nil()]).expect_err("error should raise");
    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "<no error object>");
}

#[test]
fn base_error_validates_optional_level() {
    assert_eq!(
        base_error(
            &mut TestRuntime::default(),
            &[Value::integer(1), Value::boolean(false)]
        )
        .expect_err("level should be integer")
        .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        }
    );
}

#[test]
fn base_next_returns_next_pair_or_nil() {
    let mut runtime = TestRuntime::default();
    let table = runtime.push_table(vec![
        (Value::integer(1), Value::integer(10)),
        (Value::integer(2), Value::integer(20)),
    ]);

    assert_eq!(
        call_with_runtime(&mut runtime, base_next, &[table]),
        vec![Value::integer(1), Value::integer(10)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_next, &[table, Value::integer(1)]),
        vec![Value::integer(2), Value::integer(20)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_next, &[table, Value::integer(2)]),
        vec![Value::nil()]
    );
}

#[test]
fn base_pairs_returns_next_state_nil_control_and_nil_close() {
    let mut runtime = TestRuntime {
        base_next: Value::native_function_index(7),
        ..TestRuntime::default()
    };
    let table = runtime.push_table(Vec::new());

    assert_eq!(
        call_with_runtime(&mut runtime, base_pairs, &[table]),
        vec![
            Value::native_function_index(7),
            table,
            Value::nil(),
            Value::nil()
        ]
    );
}

#[test]
fn base_pairs_calls_pairs_metamethod_and_returns_first_four_values() {
    let mut runtime = TestRuntime {
        protected_results: vec![Ok(vec![
            Value::native_function_index(8),
            Value::integer(1),
            Value::integer(2),
            Value::integer(3),
            Value::integer(4),
        ])],
        ..TestRuntime::default()
    };
    let pairs_key = runtime.push_string(b"__pairs");
    let metamethod = Value::native_function_index(9);
    let metatable = runtime.push_table(vec![(pairs_key, metamethod)]);
    let table = runtime.push_table(Vec::new());
    runtime
        .table_set_metatable(table, metatable)
        .expect("test metatable should be valid");

    assert_eq!(
        call_with_runtime(&mut runtime, base_pairs, &[table]),
        vec![
            Value::native_function_index(8),
            Value::integer(1),
            Value::integer(2),
            Value::integer(3)
        ]
    );
    assert_eq!(runtime.protected_calls, vec![(metamethod, vec![table])]);
}

#[test]
fn base_pairs_pads_pairs_metamethod_results_to_four_values() {
    let mut runtime = TestRuntime {
        protected_results: vec![Ok(vec![Value::native_function_index(8)])],
        ..TestRuntime::default()
    };
    let pairs_key = runtime.push_string(b"__pairs");
    let metamethod = Value::native_function_index(9);
    let metatable = runtime.push_table(vec![(pairs_key, metamethod)]);
    let table = runtime.push_table(Vec::new());
    runtime
        .table_set_metatable(table, metatable)
        .expect("test metatable should be valid");

    assert_eq!(
        call_with_runtime(&mut runtime, base_pairs, &[table]),
        vec![
            Value::native_function_index(8),
            Value::nil(),
            Value::nil(),
            Value::nil()
        ]
    );
}

#[test]
fn base_next_reports_non_table_receiver() {
    assert_eq!(
        base_next(&mut TestRuntime::default(), &[Value::nil()])
            .expect_err("receiver should be table")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "table",
        }
    );
}

#[test]
fn base_getmetatable_returns_nil_without_metatable() {
    let mut runtime = TestRuntime::default();
    let table = runtime.push_table(Vec::new());

    assert_eq!(
        call_with_runtime(&mut runtime, base_getmetatable, &[table]),
        vec![Value::nil()]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_getmetatable, &[Value::integer(1)]),
        vec![Value::nil()]
    );
}

#[test]
fn base_setmetatable_sets_and_clears_metatable() {
    let mut runtime = TestRuntime::default();
    let table = runtime.push_table(Vec::new());
    let metatable = runtime.push_table(Vec::new());

    assert_eq!(
        call_with_runtime(&mut runtime, base_setmetatable, &[table, metatable]),
        vec![table]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_getmetatable, &[table]),
        vec![metatable]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_setmetatable, &[table, Value::nil()]),
        vec![table]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_getmetatable, &[table]),
        vec![Value::nil()]
    );
}

#[test]
fn base_metatable_respects_protected_metatable_field() {
    let mut runtime = TestRuntime::default();
    let key = runtime.push_string(b"__metatable");
    let protected = runtime.push_string(b"locked");
    let metatable = runtime.push_table(vec![(key, protected)]);
    let replacement = runtime.push_table(Vec::new());
    let table = runtime.push_table(Vec::new());

    call_with_runtime(&mut runtime, base_setmetatable, &[table, metatable]);
    assert_eq!(
        call_with_runtime(&mut runtime, base_getmetatable, &[table]),
        vec![protected]
    );
    let error = base_setmetatable(&mut runtime, &[table, replacement])
        .expect_err("protected metatable should reject changes");
    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "cannot change a protected metatable");
}

#[test]
fn base_setmetatable_reports_type_errors() {
    assert_eq!(
        base_setmetatable(
            &mut TestRuntime::default(),
            &[Value::integer(1), Value::nil()]
        )
        .expect_err("receiver should be table")
        .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "table",
        }
    );
    let mut runtime = TestRuntime::default();
    let table = runtime.push_table(Vec::new());
    assert_eq!(
        base_setmetatable(&mut runtime, &[table, Value::integer(1)])
            .expect_err("metatable should be table or nil")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "nil or table",
        }
    );
}

#[test]
fn base_rawequal_compares_raw_values() {
    assert_eq!(
        call(base_rawequal, &[Value::integer(7), Value::float(7.0)]),
        vec![Value::boolean(true)]
    );
    assert_eq!(
        call(base_rawequal, &[Value::boolean(true), Value::integer(1)]),
        vec![Value::boolean(false)]
    );
}

#[test]
fn base_rawget_reads_raw_table_value() {
    let mut runtime = TestRuntime::default();
    let key = runtime.push_string(b"name");
    let table = runtime.push_table(vec![(key, Value::integer(42))]);

    assert_eq!(
        call_with_runtime(&mut runtime, base_rawget, &[table, key]),
        vec![Value::integer(42)]
    );
}

#[test]
fn base_rawset_writes_raw_table_value_and_returns_table() {
    let mut runtime = TestRuntime::default();
    let key = runtime.push_string(b"name");
    let table = runtime.push_table(Vec::new());

    assert_eq!(
        call_with_runtime(&mut runtime, base_rawset, &[table, key, Value::integer(42)]),
        vec![table]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_rawget, &[table, key]),
        vec![Value::integer(42)]
    );
}

#[test]
fn base_rawlen_reports_table_and_string_lengths() {
    let mut runtime = TestRuntime::default();
    let string = runtime.push_string(b"hello");
    let table = runtime.push_table(vec![
        (Value::integer(1), Value::boolean(true)),
        (Value::integer(3), Value::boolean(true)),
    ]);

    assert_eq!(
        call_with_runtime(&mut runtime, base_rawlen, &[string]),
        vec![Value::integer(5)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_rawlen, &[table]),
        vec![Value::integer(3)]
    );
}

#[test]
fn base_raw_functions_report_type_errors() {
    assert_eq!(
        base_rawget(
            &mut TestRuntime::default(),
            &[Value::integer(1), Value::integer(1)]
        )
        .expect_err("rawget receiver should be table")
        .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "table",
        }
    );
    assert_eq!(
        base_rawlen(&mut TestRuntime::default(), &[Value::integer(1)])
            .expect_err("rawlen receiver should be table or string")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "table or string",
        }
    );
}

#[test]
fn base_select_returns_positioned_arguments() {
    let values = [
        Value::integer(2),
        Value::integer(10),
        Value::integer(20),
        Value::integer(30),
    ];
    assert_eq!(
        call(base_select, &values),
        vec![Value::integer(20), Value::integer(30)]
    );

    let values = [
        Value::integer(-1),
        Value::integer(10),
        Value::integer(20),
        Value::integer(30),
    ];
    assert_eq!(call(base_select, &values), vec![Value::integer(30)]);
}

#[test]
fn base_select_hash_returns_argument_count() {
    let mut runtime = TestRuntime::default();
    let hash = runtime.push_string(b"#");
    let prefixed = runtime.push_string(b"#not-exact");

    assert_eq!(
        call_with_runtime(
            &mut runtime,
            base_select,
            &[hash, Value::integer(1), Value::nil(), Value::integer(3)]
        ),
        vec![Value::integer(3)]
    );
    assert_eq!(
        call_with_runtime(
            &mut runtime,
            base_select,
            &[prefixed, Value::integer(1), Value::nil(), Value::integer(3)]
        ),
        vec![Value::integer(3)]
    );
}

#[test]
fn base_select_reports_bad_position() {
    assert_eq!(
        base_select(&mut TestRuntime::default(), &[Value::integer(0)])
            .expect_err("zero select should fail")
            .kind(),
        &NativeErrorKind::ArgumentOutOfRange { index: 1 }
    );
}

#[test]
fn base_tonumber_returns_numbers_unchanged_without_base() {
    assert_eq!(
        call(base_tonumber, &[Value::integer(12)]),
        vec![Value::integer(12)]
    );
    assert_eq!(
        call(base_tonumber, &[Value::float(12.5)]),
        vec![Value::float(12.5)]
    );
}

#[test]
fn base_tonumber_parses_standard_string_numbers() {
    let mut runtime = TestRuntime::default();
    let integer = runtime.push_string(b" \t-42\n");
    let float = runtime.push_string(b"1.25e2");
    let hex = runtime.push_string(b"0x10");
    let positive_hex = runtime.push_string(b" +0X10 ");
    let negative_hex = runtime.push_string(b"-0x10");
    let hex_float = runtime.push_string(b"0x1.8p1");
    let fractional_hex = runtime.push_string(b"0x10.8");

    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[integer]),
        vec![Value::integer(-42)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[float]),
        vec![Value::float(125.0)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[hex]),
        vec![Value::integer(16)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[positive_hex]),
        vec![Value::integer(16)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[negative_hex]),
        vec![Value::integer(-16)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[hex_float]),
        vec![Value::float(3.0)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[fractional_hex]),
        vec![Value::float(16.5)]
    );
}

#[test]
fn base_tonumber_parses_explicit_base_integers() {
    let mut runtime = TestRuntime::default();
    let binary = runtime.push_string(b" 1010 ");
    let base36 = runtime.push_string(b"z");

    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[binary, Value::integer(2)]),
        vec![Value::integer(10)]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[base36, Value::integer(36)]),
        vec![Value::integer(35)]
    );
}

#[test]
fn base_tonumber_returns_nil_for_failed_conversion() {
    let mut runtime = TestRuntime::default();
    let invalid = runtime.push_string(b"12x");
    let invalid_base = runtime.push_string(b"2");

    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[Value::nil()]),
        vec![Value::nil()]
    );
    assert_eq!(
        call_with_runtime(&mut runtime, base_tonumber, &[invalid]),
        vec![Value::nil()]
    );
    assert_eq!(
        call_with_runtime(
            &mut runtime,
            base_tonumber,
            &[invalid_base, Value::integer(2)]
        ),
        vec![Value::nil()]
    );
}

#[test]
fn base_tonumber_reports_base_errors() {
    let mut runtime = TestRuntime::default();
    let text = runtime.push_string(b"10");

    assert_eq!(
        base_tonumber(&mut runtime, &[Value::integer(10), Value::integer(10)])
            .expect_err("base conversion requires string")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "string",
        }
    );
    assert_eq!(
        base_tonumber(&mut runtime, &[text, Value::integer(1)])
            .expect_err("base must be in range")
            .kind(),
        &NativeErrorKind::ArgumentOutOfRange { index: 2 }
    );
}

#[test]
fn base_tostring_returns_strings_unchanged() {
    let mut runtime = TestRuntime::default();
    let string = runtime.push_string(b"already");

    assert_eq!(
        call_with_runtime(&mut runtime, base_tostring, &[string]),
        vec![string]
    );
}

#[test]
fn base_tostring_formats_scalar_values() {
    let mut runtime = TestRuntime::default();

    let values = call_with_runtime(&mut runtime, base_tostring, &[Value::nil()]);
    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"nil".as_slice())
    );

    let values = call_with_runtime(&mut runtime, base_tostring, &[Value::boolean(true)]);
    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"true".as_slice())
    );

    let values = call_with_runtime(&mut runtime, base_tostring, &[Value::integer(-42)]);
    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"-42".as_slice())
    );
}

#[test]
fn base_tostring_formats_table_and_function_identities() {
    let mut runtime = TestRuntime::default();
    let table = runtime.push_table(Vec::new());

    let values = call_with_runtime(&mut runtime, base_tostring, &[table]);
    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"table: 0x0".as_slice())
    );

    let values = call_with_runtime(
        &mut runtime,
        base_tostring,
        &[Value::native_function_index(3)],
    );
    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"function: 0x3".as_slice())
    );

    let values = call_with_runtime(
        &mut runtime,
        base_tostring,
        &[Value::light_user_data(0x1234)],
    );
    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"userdata: 0x1234".as_slice())
    );
}

#[test]
fn base_type_returns_lua_type_name() {
    let mut runtime = TestRuntime::default();
    let values = call_with_runtime(&mut runtime, base_type, &[Value::integer(7)]);

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"number".as_slice())
    );

    let values = call_with_runtime(&mut runtime, base_type, &[Value::light_user_data(0x1234)]);
    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"userdata".as_slice())
    );
}
