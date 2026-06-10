//! `table.sort` native implementation.

use std::cmp::Ordering;

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

pub(super) fn table_sort(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let table = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if let Some(comparator) = args.get(1).filter(|value| !value.is_nil()) {
        if !comparator.is_closure() {
            return Err(NativeErrorKind::TypeError {
                index: 2,
                expected: "function",
            }
            .into());
        }
        return Err(NativeErrorKind::RuntimeError {
            message: "custom table.sort comparators are not supported yet".into(),
        }
        .into());
    }

    let len = runtime.table_array_len(table)?;
    if len > i64::from(i32::MAX) {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 1 }.into());
    }
    if len <= 1 {
        return Ok(Vec::new());
    }

    let mut values = Vec::with_capacity(usize::try_from(len).expect("positive i32 fits usize"));
    for index in 1..=len {
        values.push(runtime.table_get_integer(table, index)?);
    }
    sort_values(runtime, &mut values)?;
    for (offset, value) in values.into_iter().enumerate() {
        let index = i64::try_from(offset + 1).expect("sorted table index fits LuaInteger");
        runtime.table_set_integer(table, index, value)?;
    }

    Ok(Vec::new())
}

fn sort_values(runtime: &dyn NativeRuntime, values: &mut [Value]) -> Result<(), NativeError> {
    for index in 1..values.len() {
        let value = values[index];
        let mut cursor = index;
        while cursor > 0 && compare_values(runtime, value, values[cursor - 1])? == Ordering::Less {
            values[cursor] = values[cursor - 1];
            cursor -= 1;
        }
        values[cursor] = value;
    }
    Ok(())
}

fn compare_values(
    runtime: &dyn NativeRuntime,
    left: Value,
    right: Value,
) -> Result<Ordering, NativeError> {
    if let (Some(left), Some(right)) = (left.to_float(), right.to_float()) {
        return Ok(left.total_cmp(&right));
    }
    if let (Some(left), Some(right)) = (
        runtime.short_string_bytes(left),
        runtime.short_string_bytes(right),
    ) {
        return Ok(left.cmp(right));
    }
    Err(NativeErrorKind::RuntimeError {
        message: "attempt to compare values with '<'".into(),
    }
    .into())
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::table_sort;
    use crate::{NativeError, NativeErrorKind, NativeRuntime};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        tables: Vec<Vec<(Value, Value)>>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::native_function_index(index)
        }

        fn push_table(&mut self, entries: &[(Value, Value)]) -> Value {
            let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
            self.tables.push(entries.to_vec());
            Value::table_index(index)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_native_function_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }

        fn table_array_len(&self, table: Value) -> Result<i64, NativeError> {
            let table = table.as_table_index().expect("table value") as usize;
            Ok(self.tables[table]
                .iter()
                .filter_map(|(key, value)| {
                    let index = key.as_integer()?;
                    if index >= 1 && !value.is_nil() {
                        Some(index)
                    } else {
                        None
                    }
                })
                .max()
                .unwrap_or(0))
        }

        fn table_get_integer(&self, table: Value, index: i64) -> Result<Value, NativeError> {
            let table = table.as_table_index().expect("table value") as usize;
            Ok(self.tables[table]
                .iter()
                .find_map(|(key, value)| (*key == Value::integer(index)).then_some(*value))
                .unwrap_or_else(Value::nil))
        }

        fn table_set_integer(
            &mut self,
            table: Value,
            index: i64,
            value: Value,
        ) -> Result<(), NativeError> {
            let table = table.as_table_index().expect("table value") as usize;
            if let Some((_, stored)) = self.tables[table]
                .iter_mut()
                .find(|(key, _)| *key == Value::integer(index))
            {
                *stored = value;
            } else {
                self.tables[table].push((Value::integer(index), value));
            }
            Ok(())
        }
    }

    #[test]
    fn table_sort_orders_numbers_in_place() {
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(&[
            (Value::integer(1), Value::integer(3)),
            (Value::integer(2), Value::integer(1)),
            (Value::integer(3), Value::integer(2)),
        ]);

        assert_eq!(
            table_sort(&mut runtime, &[table]).expect("sort should pass"),
            Vec::<Value>::new()
        );
        assert_eq!(
            [
                runtime.table_get_integer(table, 1).expect("value"),
                runtime.table_get_integer(table, 2).expect("value"),
                runtime.table_get_integer(table, 3).expect("value"),
            ],
            [Value::integer(1), Value::integer(2), Value::integer(3)]
        );
    }

    #[test]
    fn table_sort_orders_strings_in_place() {
        let mut runtime = TestRuntime::default();
        let c = runtime.push_string(b"c");
        let a = runtime.push_string(b"a");
        let b = runtime.push_string(b"b");
        let table = runtime.push_table(&[
            (Value::integer(1), c),
            (Value::integer(2), a),
            (Value::integer(3), b),
        ]);

        table_sort(&mut runtime, &[table]).expect("sort should pass");

        let first = runtime.table_get_integer(table, 1).expect("value");
        let second = runtime.table_get_integer(table, 2).expect("value");
        let third = runtime.table_get_integer(table, 3).expect("value");
        assert_eq!(runtime.short_string_bytes(first), Some(b"a".as_slice()));
        assert_eq!(runtime.short_string_bytes(second), Some(b"b".as_slice()));
        assert_eq!(runtime.short_string_bytes(third), Some(b"c".as_slice()));
    }

    #[test]
    fn table_sort_rejects_non_function_comparator() {
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(&[]);

        assert_eq!(
            table_sort(&mut runtime, &[table, Value::integer(1)])
                .expect_err("comparator should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "function"
            }
        );
    }

    #[test]
    fn table_sort_rejects_incomparable_values() {
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(&[
            (Value::integer(1), Value::boolean(false)),
            (Value::integer(2), Value::integer(1)),
        ]);

        assert_eq!(
            table_sort(&mut runtime, &[table])
                .expect_err("incomparable value should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "attempt to compare values with '<'".into()
            }
        );
    }
}
