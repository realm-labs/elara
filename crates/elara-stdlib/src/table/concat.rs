//! `table.concat` native implementation.

use std::borrow::Cow;

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::optional_integer_arg;

pub(super) fn table_concat(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let table = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let separator = match args.get(1) {
        Some(value) if !value.is_nil() => string_or_number_arg(runtime, *value, 2)?,
        _ => Cow::Borrowed(&[] as &[u8]),
    };
    let start = optional_integer_arg(runtime, args, 3, 1)?;
    let end = optional_integer_arg(runtime, args, 4, runtime.table_array_len(table)?)?;

    let mut output = Vec::new();
    for index in start..=end {
        if index > start {
            output
                .try_reserve(separator.as_ref().len())
                .map_err(|_| concat_too_large())?;
            output.extend_from_slice(separator.as_ref());
        }
        let value = runtime.table_get_integer(table, index)?;
        let bytes = string_or_number_arg(runtime, value, 1)?;
        output
            .try_reserve(bytes.as_ref().len())
            .map_err(|_| concat_too_large())?;
        output.extend_from_slice(bytes.as_ref());
    }

    Ok(vec![runtime.intern_string(&output)?])
}

fn string_or_number_arg(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<Cow<'_, [u8]>, NativeError> {
    if let Some(bytes) = runtime.string_bytes(value) {
        return Ok(Cow::Borrowed(bytes));
    }
    if let Some(integer) = value.as_integer() {
        return Ok(Cow::Owned(integer.to_string().into_bytes()));
    }
    if let Some(float) = value.as_float() {
        return Ok(Cow::Owned(float.to_string().into_bytes()));
    }
    Err(NativeErrorKind::TypeError {
        index,
        expected: "string or number",
    }
    .into())
}

fn concat_too_large() -> NativeError {
    NativeErrorKind::RuntimeError {
        message: "resulting string too large".into(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::table_concat;
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
    }

    #[test]
    fn table_concat_joins_strings_with_separator() {
        let mut runtime = TestRuntime::default();
        let a = runtime.push_string(b"a");
        let b = runtime.push_string(b"b");
        let c = runtime.push_string(b"c");
        let separator = runtime.push_string(b"-");
        let table = runtime.push_table(&[
            (Value::integer(1), a),
            (Value::integer(2), b),
            (Value::integer(3), c),
        ]);

        let value = table_concat(&mut runtime, &[table, separator]).expect("concat should pass");

        assert_eq!(
            runtime.short_string_bytes(value[0]),
            Some(b"a-b-c".as_slice())
        );
    }

    #[test]
    fn table_concat_honors_explicit_bounds() {
        let mut runtime = TestRuntime::default();
        let a = runtime.push_string(b"a");
        let b = runtime.push_string(b"b");
        let c = runtime.push_string(b"c");
        let table = runtime.push_table(&[
            (Value::integer(1), a),
            (Value::integer(2), b),
            (Value::integer(3), c),
        ]);

        let value = table_concat(
            &mut runtime,
            &[table, Value::nil(), Value::integer(2), Value::integer(3)],
        )
        .expect("concat should pass");

        assert_eq!(runtime.short_string_bytes(value[0]), Some(b"bc".as_slice()));
    }

    #[test]
    fn table_concat_bounds_coerce_exact_numbers_and_numeric_strings() {
        let mut runtime = TestRuntime::default();
        let a = runtime.push_string(b"a");
        let b = runtime.push_string(b"b");
        let c = runtime.push_string(b"c");
        let separator = runtime.push_string(b"-");
        let start = runtime.push_string(b"2.0");
        let table = runtime.push_table(&[
            (Value::integer(1), a),
            (Value::integer(2), b),
            (Value::integer(3), c),
        ]);

        let value = table_concat(&mut runtime, &[table, separator, start, Value::float(3.0)])
            .expect("concat should pass");

        assert_eq!(
            runtime.short_string_bytes(value[0]),
            Some(b"b-c".as_slice())
        );
    }

    #[test]
    fn table_concat_concatenates_number_values() {
        let mut runtime = TestRuntime::default();
        let separator = runtime.push_string(b"-");
        let table = runtime.push_table(&[
            (Value::integer(1), Value::integer(7)),
            (Value::integer(2), Value::integer(8)),
        ]);

        let value = table_concat(&mut runtime, &[table, separator]).expect("concat should pass");

        assert_eq!(
            runtime.short_string_bytes(value[0]),
            Some(b"7-8".as_slice())
        );
    }

    #[test]
    fn table_concat_rejects_non_string_or_number_values() {
        let mut runtime = TestRuntime::default();
        let table = runtime.push_table(&[(Value::integer(1), Value::boolean(false))]);

        assert_eq!(
            table_concat(&mut runtime, &[table])
                .expect_err("non-string/non-number value should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string or number"
            }
        );
    }
}
