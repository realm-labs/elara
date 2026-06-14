use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{PRELOAD_FIELD, PRELOAD_LOADER_DATA, package_subtable};

pub(super) fn package_preload_searcher(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let name = args
        .first()
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let name_bytes = runtime
        .string_bytes(name)
        .ok_or(NativeErrorKind::TypeError {
            index: 1,
            expected: "string",
        })?
        .to_vec();
    let preload = package_subtable(runtime, PRELOAD_FIELD)?;
    let loader = runtime.table_get(preload, name)?;
    if loader.is_nil() {
        let name = String::from_utf8_lossy(&name_bytes);
        let message = format!("no field package.preload['{name}']");
        return Ok(vec![runtime.intern_string(message.as_bytes())?]);
    }

    Ok(vec![loader, runtime.intern_string(PRELOAD_LOADER_DATA)?])
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use crate::{NativeError, NativeErrorKind, NativeRuntime};

    use super::package_preload_searcher;

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        tables: Vec<Vec<(Value, Value)>>,
        globals: Vec<(Box<[u8]>, Value)>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            if let Some(index) = self
                .strings
                .iter()
                .position(|existing| existing.as_ref() == bytes)
            {
                return Value::closure_index(
                    u32::try_from(index).expect("test string index fits in u32"),
                );
            }
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::closure_index(index)
        }

        fn push_table(&mut self, entries: &[(Value, Value)]) -> Value {
            let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
            self.tables.push(entries.to_vec());
            Value::table_index(index)
        }

        fn set_global(&mut self, name: &[u8], value: Value) {
            self.globals.push((name.into(), value));
        }

        fn bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            self.bytes(value)
        }

        fn table_get(&self, table: Value, key: Value) -> Result<Value, NativeError> {
            Ok(
                self.tables[table.as_table_index().expect("test table") as usize]
                    .iter()
                    .find_map(|(entry_key, entry_value)| {
                        (*entry_key == key && !entry_value.is_nil()).then_some(*entry_value)
                    })
                    .unwrap_or_else(Value::nil),
            )
        }

        fn global_get(&mut self, name: &[u8]) -> Result<Value, NativeError> {
            Ok(self
                .globals
                .iter()
                .find_map(|(entry_name, value)| (entry_name.as_ref() == name).then_some(*value))
                .unwrap_or_else(Value::nil))
        }
    }

    #[test]
    fn package_preload_searcher_returns_loader_and_data() {
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"mod");
        let loader = Value::native_function_index(7);
        install_package_tables(&mut runtime, &[(name, loader)]);

        let result =
            package_preload_searcher(&mut runtime, &[name]).expect("preload searcher should pass");

        assert_eq!(result[0], loader);
        assert_eq!(runtime.bytes(result[1]), Some(b":preload:".as_slice()));
    }

    #[test]
    fn package_preload_searcher_returns_missing_preload_message() {
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"missing");
        install_package_tables(&mut runtime, &[]);

        let result =
            package_preload_searcher(&mut runtime, &[name]).expect("preload searcher should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(b"no field package.preload['missing']".as_slice())
        );
    }

    #[test]
    fn package_preload_searcher_validates_name_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            package_preload_searcher(&mut runtime, &[])
                .expect_err("missing name")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            package_preload_searcher(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string name")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
    }

    fn install_package_tables(runtime: &mut TestRuntime, preload_entries: &[(Value, Value)]) {
        let preload_key = runtime.push_string(b"preload");
        let preload = runtime.push_table(preload_entries);
        let package = runtime.push_table(&[(preload_key, preload)]);
        runtime.set_global(b"package", package);
    }
}
