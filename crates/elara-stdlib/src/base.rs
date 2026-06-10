//! Executable base-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable base-library functions currently implemented.
pub const BASE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "assert"), base_assert),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawequal"), base_rawequal),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawget"), base_rawget),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawlen"), base_rawlen),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "rawset"), base_rawset),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "select"), base_select),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Base, "type"), base_type),
];

fn base_assert(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let condition = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if is_truthy(condition) {
        Ok(args.to_vec())
    } else {
        Err(NativeErrorKind::LuaError.into())
    }
}

fn base_rawequal(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let left = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let right = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    Ok(vec![Value::boolean(left == right)])
}

fn base_rawget(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let table = table_arg(args, 1)?;
    let key = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    Ok(vec![runtime.table_get(table, key)?])
}

fn base_rawlen(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if let Some(bytes) = runtime.short_string_bytes(value) {
        let len = i64::try_from(bytes.len()).expect("runtime string length must fit in LuaInteger");
        return Ok(vec![Value::integer(len)]);
    }
    if value.is_table() {
        return Ok(vec![Value::integer(runtime.table_array_len(value)?)]);
    }
    Err(NativeErrorKind::TypeError {
        index: 1,
        expected: "table or string",
    }
    .into())
}

fn base_rawset(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let table = table_arg(args, 1)?;
    let key = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    let value = *args
        .get(2)
        .ok_or(NativeErrorKind::MissingArgument { index: 3 })?;
    runtime.table_set(table, key, value)?;
    Ok(vec![table])
}

fn base_select(
    _runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let index =
        args.first()
            .and_then(|value| value.as_integer())
            .ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "integer",
            })?;
    let value_count = args.len().saturating_sub(1);
    let start = select_start(index, value_count)?;
    Ok(args[start..].to_vec())
}

fn base_type(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    Ok(vec![
        runtime.intern_short_string(type_name(value).as_bytes())?,
    ])
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

fn type_name(value: Value) -> &'static str {
    if value.is_nil() {
        "nil"
    } else if value.is_bool() {
        "boolean"
    } else if value.is_number() {
        "number"
    } else if value.is_string() {
        "string"
    } else if value.is_table() {
        "table"
    } else if value.is_closure() {
        "function"
    } else {
        "unknown"
    }
}

fn table_arg(args: &[Value], index: usize) -> Result<Value, NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    if value.is_table() {
        Ok(value)
    } else {
        Err(NativeErrorKind::TypeError {
            index,
            expected: "table",
        }
        .into())
    }
}

fn select_start(index: i64, value_count: usize) -> Result<usize, NativeError> {
    let value_count = i64::try_from(value_count).expect("argument count must fit in i64");
    let normalized = if index < 0 {
        value_count + index + 1
    } else if index > value_count {
        value_count + 1
    } else {
        index
    };
    if normalized < 1 {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 1 }.into());
    }
    usize::try_from(normalized).map_err(|_| NativeErrorKind::ArgumentOutOfRange { index: 1 }.into())
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{
        BASE_NATIVE_FUNCTIONS, base_assert, base_rawequal, base_rawget, base_rawlen, base_rawset,
        base_select, base_type,
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
    fn base_type_returns_lua_type_name() {
        let mut runtime = TestRuntime::default();
        let values = call_with_runtime(&mut runtime, base_type, &[Value::integer(7)]);

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"number".as_slice())
        );
    }
}
