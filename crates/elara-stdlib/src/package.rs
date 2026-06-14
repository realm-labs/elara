//! Executable package library natives.

use std::fs::File;

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

mod loadlib;

use self::loadlib::package_loadlib;

const PATH_MARK: &str = "?";
const PATH_SEPARATOR: &str = ";";
const PACKAGE_GLOBAL: &[u8] = b"package";
const LOADED_FIELD: &[u8] = b"loaded";
const PRELOAD_FIELD: &[u8] = b"preload";
const SEARCHERS_FIELD: &[u8] = b"searchers";
const PRELOAD_LOADER_DATA: &[u8] = b":preload:";

#[cfg(windows)]
const DIRECTORY_SEPARATOR: &str = "\\";
#[cfg(not(windows))]
const DIRECTORY_SEPARATOR: &str = "/";

/// Lua `package.config` value for the current platform.
pub const PACKAGE_CONFIG: &str = if cfg!(windows) {
    "\\\n;\n?\n!\n-\n"
} else {
    "/\n;\n?\n!\n-\n"
};

/// Lua `package.path` default for the current platform.
pub const PACKAGE_PATH: &str = if cfg!(windows) {
    "!\\lua\\?.lua;!\\lua\\?\\init.lua;!\\?.lua;!\\?\\init.lua;!\\..\\share\\lua\\5.5\\?.lua;!\\..\\share\\lua\\5.5\\?\\init.lua;.\\?.lua;.\\?\\init.lua"
} else {
    "/usr/local/share/lua/5.5/?.lua;/usr/local/share/lua/5.5/?/init.lua;/usr/local/lib/lua/5.5/?.lua;/usr/local/lib/lua/5.5/?/init.lua;./?.lua;./?/init.lua"
};

/// Lua `package.cpath` default for the current platform.
pub const PACKAGE_CPATH: &str = if cfg!(windows) {
    "!\\?.dll;!\\..\\lib\\lua\\5.5\\?.dll;!\\loadall.dll;.\\?.dll"
} else {
    "/usr/local/lib/lua/5.5/?.so;/usr/local/lib/lua/5.5/loadall.so;./?.so"
};

/// Executable `package` library functions currently implemented.
pub const PACKAGE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Package, "loadlib"),
        package_loadlib,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Package, "searchpath"),
        package_searchpath,
    ),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Package, "require"),
        package_require,
    ),
];

fn package_searchpath(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let name = utf8_string_arg(runtime, args, 1)?;
    let path = utf8_string_arg(runtime, args, 2)?;
    let separator = optional_utf8_string_arg(runtime, args, 3)?.unwrap_or(".");
    let directory_separator =
        optional_utf8_string_arg(runtime, args, 4)?.unwrap_or(DIRECTORY_SEPARATOR);

    let normalized_name = if !separator.is_empty() && name.contains(separator) {
        name.replace(separator, directory_separator)
    } else {
        name.to_owned()
    };
    let expanded_path = path.replace(PATH_MARK, &normalized_name);

    for filename in expanded_path.split(PATH_SEPARATOR) {
        if readable(filename) {
            return Ok(vec![runtime.intern_string(filename.as_bytes())?]);
        }
    }

    let message = format!(
        "no file '{}'",
        expanded_path.replace(PATH_SEPARATOR, "'\n\tno file '")
    );
    Ok(vec![
        Value::nil(),
        runtime.intern_string(message.as_bytes())?,
    ])
}

fn package_require(
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

    let loaded = package_subtable(runtime, LOADED_FIELD)?;
    let current = runtime.table_get(loaded, name)?;
    if is_truthy(current) {
        return Ok(vec![current]);
    }

    if let Some((loader, loader_data)) = searcher_loader(runtime, name)? {
        return run_loader(runtime, loaded, name, loader, loader_data);
    }

    let preload = package_subtable(runtime, PRELOAD_FIELD)?;
    let loader = runtime.table_get(preload, name)?;
    if !loader.is_closure() {
        let name = String::from_utf8_lossy(&name_bytes);
        return Err(NativeError::lua_error(format!(
            "module '{name}' not found:\n\tno field package.preload['{name}']"
        )));
    }

    let loader_data = runtime.intern_string(PRELOAD_LOADER_DATA)?;
    run_loader(runtime, loaded, name, loader, loader_data)
}

fn searcher_loader(
    runtime: &mut dyn NativeRuntime,
    name: Value,
) -> Result<Option<(Value, Value)>, NativeError> {
    let searchers = package_subtable(runtime, SEARCHERS_FIELD)?;
    let mut index = 1;
    loop {
        let searcher = runtime.table_get(searchers, Value::integer(index))?;
        if searcher.is_nil() {
            return Ok(None);
        }
        if !searcher.is_closure() {
            return Err(NativeError::lua_error(format!(
                "package.searchers[{index}] is not a function"
            )));
        }
        let results = match runtime.protected_call(searcher, &[name])? {
            Ok(results) => results,
            Err(message) => return Err(NativeError::lua_error(message)),
        };
        if let Some(loader) = results.first().copied().filter(|value| value.is_closure()) {
            let loader_data = results.get(1).copied().unwrap_or_else(Value::nil);
            return Ok(Some((loader, loader_data)));
        }
        index += 1;
    }
}

fn run_loader(
    runtime: &mut dyn NativeRuntime,
    loaded: Value,
    name: Value,
    loader: Value,
    loader_data: Value,
) -> Result<Vec<Value>, NativeError> {
    let loaded_value = match runtime.protected_call(loader, &[name, loader_data])? {
        Ok(results) => results.first().copied().unwrap_or_else(Value::nil),
        Err(message) => return Err(NativeError::lua_error(message)),
    };
    if !loaded_value.is_nil() {
        runtime.table_set(loaded, name, loaded_value)?;
    }

    let module = runtime.table_get(loaded, name)?;
    let module = if module.is_nil() {
        let value = Value::boolean(true);
        runtime.table_set(loaded, name, value)?;
        value
    } else {
        module
    };

    Ok(vec![module, loader_data])
}

fn package_subtable(runtime: &mut dyn NativeRuntime, field: &[u8]) -> Result<Value, NativeError> {
    let package = runtime.global_get(PACKAGE_GLOBAL)?;
    let key = runtime.intern_short_string(field)?;
    let value = runtime.table_get(package, key)?;
    if value.is_table() {
        Ok(value)
    } else {
        Err(NativeErrorKind::RuntimeError {
            message: format!("package.{} must be a table", String::from_utf8_lossy(field))
                .into_boxed_str(),
        }
        .into())
    }
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

fn readable(filename: &str) -> bool {
    File::open(filename).is_ok()
}

fn utf8_string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<&'a str, NativeError> {
    let bytes = string_arg(runtime, args, index)?;
    std::str::from_utf8(bytes)
        .map_err(|_| NativeErrorKind::TypeError {
            index,
            expected: "utf-8 string",
        })
        .map_err(Into::into)
}

fn optional_utf8_string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Option<&'a str>, NativeError> {
    let Some(value) = args.get(index - 1).copied() else {
        return Ok(None);
    };
    if value.is_nil() {
        return Ok(None);
    }
    let bytes = runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string",
        })?;
    std::str::from_utf8(bytes)
        .map(Some)
        .map_err(|_| NativeErrorKind::TypeError {
            index,
            expected: "utf-8 string",
        })
        .map_err(Into::into)
}

fn string_arg<'a>(
    runtime: &'a dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<&'a [u8], NativeError> {
    let value = args
        .get(index - 1)
        .copied()
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    runtime
        .string_bytes(value)
        .ok_or(NativeErrorKind::TypeError {
            index,
            expected: "string",
        })
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use elara_core::Value;

    use crate::{NativeErrorKind, StdLib, native_functions};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        tables: Vec<Vec<(Value, Value)>>,
        globals: Vec<(Box<[u8]>, Value)>,
        protected_calls: Vec<(Value, Vec<Value>)>,
        protected_results: Vec<Result<Vec<Value>, Box<str>>>,
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

        fn set_table(&mut self, table: Value, key: Value, value: Value) {
            let table = &mut self.tables[table.as_table_index().expect("test table") as usize];
            if let Some((_, entry)) = table.iter_mut().find(|(entry_key, _)| *entry_key == key) {
                *entry = value;
            } else {
                table.push((key, value));
            }
        }

        fn get_table(&self, table: Value, key: Value) -> Value {
            self.tables[table.as_table_index().expect("test table") as usize]
                .iter()
                .find_map(|(entry_key, entry_value)| {
                    (*entry_key == key && !entry_value.is_nil()).then_some(*entry_value)
                })
                .unwrap_or_else(Value::nil)
        }

        fn bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    impl crate::NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, crate::NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            self.bytes(value)
        }

        fn table_get(&self, table: Value, key: Value) -> Result<Value, crate::NativeError> {
            Ok(self.get_table(table, key))
        }

        fn table_set(
            &mut self,
            table: Value,
            key: Value,
            value: Value,
        ) -> Result<(), crate::NativeError> {
            self.set_table(table, key, value);
            Ok(())
        }

        fn global_get(&mut self, name: &[u8]) -> Result<Value, crate::NativeError> {
            Ok(self
                .globals
                .iter()
                .find_map(|(entry_name, value)| (entry_name.as_ref() == name).then_some(*value))
                .unwrap_or_else(Value::nil))
        }

        fn protected_call(
            &mut self,
            function: Value,
            args: &[Value],
        ) -> Result<Result<Vec<Value>, Box<str>>, crate::NativeError> {
            self.protected_calls.push((function, args.to_vec()));
            Ok(self.protected_results.remove(0))
        }
    }

    #[test]
    fn package_searchpath_finds_existing_path_template() {
        let function = function("searchpath");
        let mut runtime = TestRuntime::default();
        let directory = unique_temp_dir();
        fs::create_dir_all(&directory).expect("test directory should be created");
        let filename = directory.join("mod.lua");
        fs::write(&filename, b"return true").expect("test file should be written");
        let template = directory.join("?.lua");
        let name = runtime.push_string(b"mod");
        let path = runtime.push_string(template.to_string_lossy().as_bytes());

        let result = function(&mut runtime, &[name, path]).expect("searchpath should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(filename.to_string_lossy().as_bytes())
        );
        fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn package_searchpath_replaces_module_separators() {
        let function = function("searchpath");
        let mut runtime = TestRuntime::default();
        let directory = unique_temp_dir();
        let nested = directory.join("a");
        fs::create_dir_all(&nested).expect("test directory should be created");
        let filename = nested.join("b.lua");
        fs::write(&filename, b"return true").expect("test file should be written");
        let template = directory.join("?.lua");
        let name = runtime.push_string(b"a.b");
        let path = runtime.push_string(template.to_string_lossy().as_bytes());

        let result = function(&mut runtime, &[name, path]).expect("searchpath should pass");

        assert_eq!(
            runtime.bytes(result[0]),
            Some(filename.to_string_lossy().as_bytes())
        );
        fs::remove_dir_all(directory).expect("test directory should be removed");
    }

    #[test]
    fn package_searchpath_returns_nil_and_error_when_absent() {
        let function = function("searchpath");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"missing");
        let path = runtime.push_string(b"./?.lua;./?/init.lua");

        let result = function(&mut runtime, &[name, path]).expect("searchpath should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"no file './missing.lua'\n\tno file './missing/init.lua'".as_slice())
        );
    }

    #[test]
    fn package_searchpath_validates_arguments() {
        let function = function("searchpath");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"mod");

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing name")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[name])
                .expect_err("missing path")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 2 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1), name])
                .expect_err("non-string name")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
    }

    #[test]
    fn package_require_loads_preloaded_module_and_caches_result() {
        let function = function("require");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"mod");
        let loader = Value::native_function_index(7);
        let loaded = runtime.push_table(&[]);
        let preload = runtime.push_table(&[(name, loader)]);
        let searchers = runtime.push_table(&[]);
        install_package_tables(&mut runtime, loaded, preload, searchers);
        runtime.protected_results.push(Ok(vec![Value::integer(42)]));

        let result = function(&mut runtime, &[name]).expect("require should load");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], Value::integer(42));
        assert_eq!(runtime.bytes(result[1]), Some(b":preload:".as_slice()));
        assert_eq!(
            runtime.protected_calls,
            vec![(loader, vec![name, result[1]])]
        );
        assert_eq!(runtime.get_table(loaded, name), Value::integer(42));
    }

    #[test]
    fn package_require_loads_module_from_custom_searcher() {
        let function = function("require");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"mod");
        let searcher = Value::native_function_index(5);
        let loader = Value::native_function_index(7);
        let loader_data = runtime.push_string(b"custom");
        let loaded = runtime.push_table(&[]);
        let preload = runtime.push_table(&[]);
        let searchers = runtime.push_table(&[(Value::integer(1), searcher)]);
        install_package_tables(&mut runtime, loaded, preload, searchers);
        runtime
            .protected_results
            .push(Ok(vec![loader, loader_data]));
        runtime.protected_results.push(Ok(vec![Value::integer(42)]));

        let result = function(&mut runtime, &[name]).expect("require should load");

        assert_eq!(result, vec![Value::integer(42), loader_data]);
        assert_eq!(
            runtime.protected_calls,
            vec![(searcher, vec![name]), (loader, vec![name, loader_data])]
        );
        assert_eq!(runtime.get_table(loaded, name), Value::integer(42));
    }

    #[test]
    fn package_require_uses_true_when_loader_returns_nil() {
        let function = function("require");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"mod");
        let loader = Value::native_function_index(7);
        let loaded = runtime.push_table(&[]);
        let preload = runtime.push_table(&[(name, loader)]);
        let searchers = runtime.push_table(&[]);
        install_package_tables(&mut runtime, loaded, preload, searchers);
        runtime.protected_results.push(Ok(vec![Value::nil()]));

        let result = function(&mut runtime, &[name]).expect("require should load");

        assert_eq!(result[0], Value::boolean(true));
        assert_eq!(runtime.get_table(loaded, name), Value::boolean(true));
    }

    #[test]
    fn package_require_returns_cached_truthy_value_without_loader() {
        let function = function("require");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"mod");
        let loaded = runtime.push_table(&[(name, Value::integer(99))]);
        let preload = runtime.push_table(&[]);
        let searchers = runtime.push_table(&[]);
        install_package_tables(&mut runtime, loaded, preload, searchers);

        let result = function(&mut runtime, &[name]).expect("require should use cache");

        assert_eq!(result, vec![Value::integer(99)]);
        assert!(runtime.protected_calls.is_empty());
    }

    #[test]
    fn package_require_reports_missing_preload() {
        let function = function("require");
        let mut runtime = TestRuntime::default();
        let name = runtime.push_string(b"missing");
        let loaded = runtime.push_table(&[]);
        let preload = runtime.push_table(&[]);
        let searchers = runtime.push_table(&[]);
        install_package_tables(&mut runtime, loaded, preload, searchers);

        let error = function(&mut runtime, &[name]).expect_err("missing module should error");

        assert_eq!(error.kind(), &NativeErrorKind::LuaError);
        assert_eq!(
            error.message(),
            "module 'missing' not found:\n\tno field package.preload['missing']"
        );
    }

    #[test]
    fn package_require_validates_name_argument() {
        let function = function("require");
        let mut runtime = TestRuntime::default();

        assert_eq!(
            function(&mut runtime, &[])
                .expect_err("missing name")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            function(&mut runtime, &[Value::integer(1)])
                .expect_err("non-string name")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
    }

    fn function(name: &str) -> crate::NativeStdFunction {
        native_functions(StdLib::Package)
            .iter()
            .find(|function| function.descriptor().name() == name)
            .expect("package native function should exist")
            .function()
    }

    fn install_package_tables(
        runtime: &mut TestRuntime,
        loaded: Value,
        preload: Value,
        searchers: Value,
    ) {
        let loaded_key = runtime.push_string(b"loaded");
        let preload_key = runtime.push_string(b"preload");
        let searchers_key = runtime.push_string(b"searchers");
        let package = runtime.push_table(&[
            (loaded_key, loaded),
            (preload_key, preload),
            (searchers_key, searchers),
        ]);
        runtime.set_global(b"package", package);
    }

    fn unique_temp_dir() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test time should be after Unix epoch")
            .as_nanos();
        let count = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("elara-package-searchpath-{suffix}-{count}"))
    }
}
