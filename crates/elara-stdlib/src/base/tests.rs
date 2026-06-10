use elara_core::Value;

use super::{
    BASE_NATIVE_FUNCTIONS, base_assert, base_rawequal, base_rawget, base_rawlen, base_rawset,
    base_select, base_tonumber, base_type,
};
use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

#[derive(Default)]
struct TestRuntime {
    strings: Vec<Box<[u8]>>,
    tables: Vec<Vec<(Value, Value)>>,
}

impl TestRuntime {
    fn push_string(&mut self, bytes: &[u8]) -> Value {
        let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
        self.strings.push(bytes.into());
        Value::closure_index(index)
    }

    fn push_table(&mut self, entries: Vec<(Value, Value)>) -> Value {
        let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
        self.tables.push(entries);
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
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawequal")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawget")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawlen")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "rawset")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "select")));
    assert!(descriptors.contains(&FunctionSpec::new(StdLib::Base, "tonumber")));
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
fn base_type_returns_lua_type_name() {
    let mut runtime = TestRuntime::default();
    let values = call_with_runtime(&mut runtime, base_type, &[Value::integer(7)]);

    assert_eq!(
        runtime.short_string_bytes(values[0]),
        Some(b"number".as_slice())
    );
}
