//! Executable table-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

mod concat;
mod sort;

use concat::table_concat;
use sort::table_sort;

/// Executable table-library functions currently implemented.
pub const TABLE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Table, "concat"), table_concat),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Table, "insert"), table_insert),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Table, "move"), table_move),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Table, "pack"), table_pack),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Table, "remove"), table_remove),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Table, "sort"), table_sort),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Table, "unpack"), table_unpack),
];

fn table_insert(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let table = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let end = runtime
        .table_array_len(table)?
        .checked_add(1)
        .ok_or_else(|| NativeErrorKind::RuntimeError {
            message: "table length overflow".into(),
        })?;

    let (position, value) = match args.len() {
        2 => (end, args[1]),
        3 => {
            let position = integer_arg(args, 2)?;
            if position < 1 || position > end {
                return Err(NativeErrorKind::ArgumentOutOfRange { index: 2 }.into());
            }
            for index in ((position + 1)..=end).rev() {
                let previous = runtime.table_get_integer(table, index - 1)?;
                runtime.table_set_integer(table, index, previous)?;
            }
            (position, args[2])
        }
        _ => {
            return Err(NativeErrorKind::RuntimeError {
                message: "wrong number of arguments to 'insert'".into(),
            }
            .into());
        }
    };

    runtime.table_set_integer(table, position, value)?;
    Ok(Vec::new())
}

fn table_move(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let source = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let first = integer_arg(args, 2)?;
    let last = integer_arg(args, 3)?;
    let target = integer_arg(args, 4)?;
    let destination = args
        .get(4)
        .copied()
        .filter(|value| !value.is_nil())
        .unwrap_or(source);

    if last >= first {
        let count = last
            .checked_sub(first)
            .and_then(|n| n.checked_add(1))
            .ok_or(NativeErrorKind::RuntimeError {
                message: "too many elements to move".into(),
            })?;
        target
            .checked_add(count - 1)
            .ok_or(NativeErrorKind::RuntimeError {
                message: "destination wrap around".into(),
            })?;

        let copy_forward = target > last || target <= first || destination != source;
        if copy_forward {
            for offset in 0..count {
                let value = runtime.table_get_integer(source, first + offset)?;
                runtime.table_set_integer(destination, target + offset, value)?;
            }
        } else {
            for offset in (0..count).rev() {
                let value = runtime.table_get_integer(source, first + offset)?;
                runtime.table_set_integer(destination, target + offset, value)?;
            }
        }
    }

    Ok(vec![destination])
}

fn table_remove(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let table = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;

    let size = runtime.table_array_len(table)?;
    let mut position = optional_integer_arg(args, 2, size)?;
    if position != size && (position < 1 || position > size + 1) {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 2 }.into());
    }

    let result = runtime.table_get_integer(table, position)?;
    while position < size {
        let next = runtime.table_get_integer(table, position + 1)?;
        runtime.table_set_integer(table, position, next)?;
        position += 1;
    }
    runtime.table_set_integer(table, position, Value::nil())?;
    Ok(vec![result])
}

fn table_pack(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let count = i64::try_from(args.len()).map_err(|_| NativeErrorKind::RuntimeError {
        message: "too many arguments to pack".into(),
    })?;
    let n_key = runtime.intern_short_string(b"n")?;
    let mut entries = Vec::with_capacity(args.len() + 1);
    for (index, value) in args.iter().copied().enumerate() {
        let key = i64::try_from(index + 1).expect("argument count already fits in LuaInteger");
        entries.push((Value::integer(key), value));
    }
    entries.push((n_key, Value::integer(count)));
    Ok(vec![runtime.create_table(&entries)?])
}

fn table_unpack(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let table = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let start = optional_integer_arg(args, 2, 1)?;
    let end = match args.get(2) {
        Some(_) => optional_integer_arg(args, 3, 0)?,
        None => runtime.table_array_len(table)?,
    };

    if start > end {
        return Ok(Vec::new());
    }

    let count = i128::from(end) - i128::from(start) + 1;
    if count > i128::from(i32::MAX) {
        return Err(NativeErrorKind::RuntimeError {
            message: "too many results to unpack".into(),
        }
        .into());
    }
    let count = usize::try_from(count).expect("positive count under i32::MAX fits usize");

    let mut values = Vec::with_capacity(count);
    for offset in 0..count {
        let index = start
            .checked_add(i64::try_from(offset).expect("offset under i32::MAX fits LuaInteger"))
            .expect("unpack index must not overflow LuaInteger");
        values.push(runtime.table_get_integer(table, index)?);
    }
    Ok(values)
}

fn integer_arg(args: &[Value], index: usize) -> Result<i64, NativeError> {
    args.get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?
        .as_integer()
        .ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "integer",
            }
            .into(),
        )
}

fn optional_integer_arg(args: &[Value], index: usize, default: i64) -> Result<i64, NativeError> {
    match args.get(index - 1) {
        Some(value) if value.is_nil() => Ok(default),
        Some(value) => value.as_integer().ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "integer",
            }
            .into(),
        ),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{
        TABLE_NATIVE_FUNCTIONS, table_insert, table_move, table_pack, table_remove, table_unpack,
    };
    use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        tables: Vec<Vec<(Value, Value)>>,
    }

    impl TestRuntime {
        fn table_entries(&self, value: Value) -> &[(Value, Value)] {
            let index = value.as_table_index().expect("table value") as usize;
            &self.tables[index]
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Ok(Value::table_index(index))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_table_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }

        fn create_table(&mut self, entries: &[(Value, Value)]) -> Result<Value, NativeError> {
            let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
            self.tables.push(entries.to_vec());
            Ok(Value::table_index(index))
        }

        fn table_array_len(&self, table: Value) -> Result<i64, NativeError> {
            let entries = self.table_entries(table);
            let len = entries
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
                .unwrap_or(0);
            Ok(len)
        }

        fn table_get_integer(&self, table: Value, index: i64) -> Result<Value, NativeError> {
            Ok(self
                .table_entries(table)
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
            let entries = {
                let table_index = table.as_table_index().expect("table value") as usize;
                &mut self.tables[table_index]
            };
            if let Some((_, existing)) = entries
                .iter_mut()
                .find(|(key, _)| *key == Value::integer(index))
            {
                *existing = value;
            } else {
                entries.push((Value::integer(index), value));
            }
            Ok(())
        }
    }

    #[test]
    fn table_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = TABLE_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "concat")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "insert")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "move")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "pack")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "remove")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "sort")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "unpack")));
    }

    #[test]
    fn table_pack_creates_array_entries_and_count_field() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::nil(), Value::integer(3)],
        )
        .expect("pack should pass");
        let entries = runtime.table_entries(packed[0]);

        assert!(entries.contains(&(Value::integer(1), Value::integer(1))));
        assert!(entries.contains(&(Value::integer(2), Value::nil())));
        assert!(entries.contains(&(Value::integer(3), Value::integer(3))));

        let n_key = entries
            .iter()
            .find(|(key, _)| runtime.short_string_bytes(*key) == Some(b"n".as_slice()))
            .expect("n field should be present")
            .0;
        assert!(entries.contains(&(n_key, Value::integer(3))));
    }

    #[test]
    fn table_insert_appends_when_called_with_value_only() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(&mut runtime, &[Value::integer(1)]).expect("pack should pass");

        assert_eq!(
            table_insert(&mut runtime, &[packed[0], Value::integer(2)])
                .expect("insert should pass"),
            Vec::<Value>::new()
        );
        assert_eq!(
            table_unpack(&mut runtime, &[packed[0]]).expect("unpack should pass"),
            vec![Value::integer(1), Value::integer(2)]
        );
    }

    #[test]
    fn table_insert_shifts_values_at_explicit_position() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(&mut runtime, &[Value::integer(1), Value::integer(3)])
            .expect("pack should pass");

        table_insert(
            &mut runtime,
            &[packed[0], Value::integer(2), Value::integer(2)],
        )
        .expect("insert should pass");

        assert_eq!(
            table_unpack(&mut runtime, &[packed[0]]).expect("unpack should pass"),
            vec![Value::integer(1), Value::integer(2), Value::integer(3)]
        );
    }

    #[test]
    fn table_insert_rejects_out_of_bounds_position() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(&mut runtime, &[]).expect("pack should pass");

        assert_eq!(
            table_insert(
                &mut runtime,
                &[packed[0], Value::integer(2), Value::integer(7)]
            )
            .expect_err("position should fail")
            .kind(),
            &NativeErrorKind::ArgumentOutOfRange { index: 2 }
        );
    }

    #[test]
    fn table_move_copies_range_to_destination_table() {
        let mut runtime = TestRuntime::default();
        let source = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::integer(2), Value::integer(3)],
        )
        .expect("pack should pass");
        let destination = table_pack(&mut runtime, &[]).expect("pack should pass");

        let moved = table_move(
            &mut runtime,
            &[
                source[0],
                Value::integer(2),
                Value::integer(3),
                Value::integer(1),
                destination[0],
            ],
        )
        .expect("move should pass");

        assert_eq!(moved, vec![destination[0]]);
        assert_eq!(
            table_unpack(&mut runtime, &[destination[0]]).expect("unpack should pass"),
            vec![Value::integer(2), Value::integer(3)]
        );
    }

    #[test]
    fn table_move_copies_backward_for_overlapping_same_table_range() {
        let mut runtime = TestRuntime::default();
        let table = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::integer(2), Value::integer(3)],
        )
        .expect("pack should pass");

        table_move(
            &mut runtime,
            &[
                table[0],
                Value::integer(1),
                Value::integer(2),
                Value::integer(2),
            ],
        )
        .expect("move should pass");

        assert_eq!(
            table_unpack(&mut runtime, &[table[0]]).expect("unpack should pass"),
            vec![Value::integer(1), Value::integer(1), Value::integer(2)]
        );
    }

    #[test]
    fn table_remove_removes_last_value_by_default() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::integer(2), Value::integer(3)],
        )
        .expect("pack should pass");

        assert_eq!(
            table_remove(&mut runtime, &[packed[0]]).expect("remove should pass"),
            vec![Value::integer(3)]
        );
        assert_eq!(
            table_unpack(&mut runtime, &[packed[0]]).expect("unpack should pass"),
            vec![Value::integer(1), Value::integer(2)]
        );
    }

    #[test]
    fn table_remove_shifts_values_after_explicit_position() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::integer(2), Value::integer(3)],
        )
        .expect("pack should pass");

        assert_eq!(
            table_remove(&mut runtime, &[packed[0], Value::integer(2)])
                .expect("remove should pass"),
            vec![Value::integer(2)]
        );
        assert_eq!(
            table_unpack(&mut runtime, &[packed[0]]).expect("unpack should pass"),
            vec![Value::integer(1), Value::integer(3)]
        );
    }

    #[test]
    fn table_remove_ignores_extra_arguments() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::integer(2), Value::integer(3)],
        )
        .expect("pack should pass");

        assert_eq!(
            table_remove(
                &mut runtime,
                &[packed[0], Value::integer(2), Value::boolean(false)]
            )
            .expect("remove should pass"),
            vec![Value::integer(2)]
        );
        assert_eq!(
            table_unpack(&mut runtime, &[packed[0]]).expect("unpack should pass"),
            vec![Value::integer(1), Value::integer(3)]
        );
    }

    #[test]
    fn table_remove_empty_table_returns_nil() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(&mut runtime, &[]).expect("pack should pass");

        assert_eq!(
            table_remove(&mut runtime, &[packed[0]]).expect("remove should pass"),
            vec![Value::nil()]
        );
    }

    #[test]
    fn default_native_runtime_rejects_table_allocation() {
        #[derive(Default)]
        struct StringOnlyRuntime;

        impl NativeRuntime for StringOnlyRuntime {
            fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, NativeError> {
                Ok(Value::integer(1))
            }

            fn short_string_bytes(&self, _value: Value) -> Option<&[u8]> {
                None
            }
        }

        let error = table_pack(&mut StringOnlyRuntime, &[])
            .expect_err("table allocation should fail")
            .kind()
            .clone();
        assert_eq!(
            error,
            NativeErrorKind::RuntimeError {
                message: "native runtime does not support table allocation".into()
            }
        );
    }

    #[test]
    fn table_unpack_returns_range_values() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::nil(), Value::integer(3)],
        )
        .expect("pack should pass");

        assert_eq!(
            table_unpack(&mut runtime, &[packed[0]]).expect("unpack should pass"),
            vec![Value::integer(1), Value::nil(), Value::integer(3)]
        );
    }

    #[test]
    fn table_unpack_honors_explicit_bounds() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(
            &mut runtime,
            &[Value::integer(1), Value::integer(2), Value::integer(3)],
        )
        .expect("pack should pass");

        assert_eq!(
            table_unpack(
                &mut runtime,
                &[packed[0], Value::integer(2), Value::integer(3)]
            )
            .expect("unpack should pass"),
            vec![Value::integer(2), Value::integer(3)]
        );
    }

    #[test]
    fn table_unpack_empty_range_returns_no_values() {
        let mut runtime = TestRuntime::default();
        let packed = table_pack(&mut runtime, &[Value::integer(1)]).expect("pack should pass");

        assert_eq!(
            table_unpack(
                &mut runtime,
                &[packed[0], Value::integer(2), Value::integer(1)]
            )
            .expect("unpack should pass"),
            Vec::<Value>::new()
        );
    }
}
