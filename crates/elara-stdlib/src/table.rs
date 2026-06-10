//! Executable table-library natives.

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

/// Executable table-library functions currently implemented.
pub const TABLE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Table, "pack"),
    table_pack,
)];

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

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::{TABLE_NATIVE_FUNCTIONS, table_pack};
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
    }

    #[test]
    fn table_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = TABLE_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Table, "pack")));
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
}
