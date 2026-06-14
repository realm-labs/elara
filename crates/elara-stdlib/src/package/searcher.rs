use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{
    PACKAGE_GLOBAL, PATH_FIELD, PRELOAD_FIELD, PRELOAD_LOADER_DATA, package_searchpath,
    package_subtable,
};

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

pub(super) fn package_lua_searcher(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let name = string_arg(runtime, args, 1)?;
    let path = package_field(runtime, PATH_FIELD)?;
    let result = package_searchpath(runtime, &[name, path])?;
    let Some(filename) = result.first().copied().filter(|value| !value.is_nil()) else {
        return Ok(vec![result.get(1).copied().unwrap_or_else(Value::nil)]);
    };
    let filename = runtime
        .string_bytes(filename)
        .ok_or(NativeErrorKind::RuntimeError {
            message: "package.searchpath returned a non-string filename".into(),
        })?;
    let message = format!(
        "loading Lua file '{}' is not supported by this runtime",
        String::from_utf8_lossy(filename)
    );
    Ok(vec![runtime.intern_string(message.as_bytes())?])
}

fn string_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Value, NativeError> {
    let value = args
        .get(index - 1)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    if runtime.string_bytes(value).is_some() {
        Ok(value)
    } else {
        Err(NativeErrorKind::TypeError {
            index,
            expected: "string",
        }
        .into())
    }
}

fn package_field(runtime: &mut dyn NativeRuntime, field: &[u8]) -> Result<Value, NativeError> {
    let package = runtime.global_get(PACKAGE_GLOBAL)?;
    let key = runtime.intern_short_string(field)?;
    runtime.table_get(package, key)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use elara_core::Value;

    use crate::{NativeError, NativeErrorKind, NativeRuntime};

    use super::{package_lua_searcher, package_preload_searcher};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    #[test]
    fn package_lua_searcher_returns_path_miss_message() {
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"missing");
        let path = runtime.push_string(b"./?.lua");
        let path_key = runtime.push_string(b"path");
        install_package_table(&mut runtime, &[(path_key, path)]);

        let result = package_lua_searcher(&mut runtime, &[name]).expect("Lua searcher should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(b"no file './missing.lua'".as_slice())
        );
    }

    #[test]
    fn package_lua_searcher_reports_unsupported_file_loading() {
        let mut runtime = TestRuntime::default();
        let directory = unique_temp_dir();
        fs::create_dir_all(&directory).expect("test directory should be created");
        let filename = directory.join("mod.lua");
        fs::write(&filename, b"return true").expect("test file should be written");
        let template = directory.join("?.lua");
        let name = runtime.push_string(b"mod");
        let path = runtime.push_string(template.to_string_lossy().as_bytes());
        let path_key = runtime.push_string(b"path");
        install_package_table(&mut runtime, &[(path_key, path)]);

        let result = package_lua_searcher(&mut runtime, &[name]).expect("Lua searcher should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(
                format!(
                    "loading Lua file '{}' is not supported by this runtime",
                    filename.to_string_lossy()
                )
                .as_bytes()
            )
        );
        fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn package_lua_searcher_validates_name_argument() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            package_lua_searcher(&mut runtime, &[])
                .expect_err("missing name")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            package_lua_searcher(&mut runtime, &[Value::integer(1)])
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
        install_package_table(runtime, &[(preload_key, preload)]);
    }

    fn install_package_table(runtime: &mut TestRuntime, entries: &[(Value, Value)]) {
        let package = runtime.push_table(entries);
        runtime.set_global(b"package", package);
    }

    fn unique_temp_dir() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test time should be after Unix epoch")
            .as_nanos();
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("elara-package-lua-searcher-{suffix}-{count}"))
    }
}
