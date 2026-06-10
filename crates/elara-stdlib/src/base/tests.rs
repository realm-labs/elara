use elara_core::Value;

use super::{
    BASE_NATIVE_FUNCTIONS, base_assert, base_error, base_getmetatable, base_next, base_print,
    base_rawequal, base_rawget, base_rawlen, base_rawset, base_select, base_setmetatable,
    base_tonumber, base_tostring, base_type,
};
use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

#[derive(Default)]
struct TestRuntime {
    strings: Vec<Box<[u8]>>,
    tables: Vec<Vec<(Value, Value)>>,
    metatables: Vec<Option<Value>>,
    output: Vec<u8>,
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
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "error")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "getmetatable")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "next")));
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
    assert_eq!(
        base_assert(&mut TestRuntime::default(), &[Value::boolean(false)])
            .expect_err("false assert should fail")
            .kind(),
        &NativeErrorKind::LuaError
    );
    assert_eq!(
        base_assert(&mut TestRuntime::default(), &[Value::nil()])
            .expect_err("nil assert should fail")
            .kind(),
        &NativeErrorKind::LuaError
    );
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
    assert_eq!(error.message(), "nil");

    let error =
        base_error(&mut TestRuntime::default(), &[Value::nil()]).expect_err("error should raise");
    assert_eq!(error.kind(), &NativeErrorKind::LuaError);
    assert_eq!(error.message(), "nil");
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
}

#[test]
fn base_type_returns_lua_type_name() {
    let mut runtime = TestRuntime::default();
    let values = call_with_runtime(&mut runtime, base_type, &[Value::integer(7)]);

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"number".as_slice())
    );
}
