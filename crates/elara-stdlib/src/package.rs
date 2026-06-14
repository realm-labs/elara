//! Executable package library natives.

use std::{fs::File, path::MAIN_SEPARATOR_STR};

use elara_core::Value;

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
};

const PATH_MARK: &str = "?";
const PATH_SEPARATOR: &str = ";";

/// Executable `package` library functions currently implemented.
pub const PACKAGE_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[NativeFunctionSpec::new(
    FunctionSpec::new(StdLib::Package, "searchpath"),
    package_searchpath,
)];

fn package_searchpath(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let name = utf8_string_arg(runtime, args, 1)?;
    let path = utf8_string_arg(runtime, args, 2)?;
    let separator = optional_utf8_string_arg(runtime, args, 3)?.unwrap_or(".");
    let directory_separator =
        optional_utf8_string_arg(runtime, args, 4)?.unwrap_or(MAIN_SEPARATOR_STR);

    let normalized_name = if !separator.is_empty() && name.contains(separator) {
        name.replace(separator, directory_separator)
    } else {
        name.to_owned()
    };
    let expanded_path = path.replace(PATH_MARK, &normalized_name);

    for filename in expanded_path.split(PATH_SEPARATOR) {
        if readable(filename) {
            return Ok(vec![runtime.intern_short_string(filename.as_bytes())?]);
        }
    }

    let message = format!(
        "no file '{}'",
        expanded_path.replace(PATH_SEPARATOR, "'\n\tno file '")
    );
    Ok(vec![
        Value::nil(),
        runtime.intern_short_string(message.as_bytes())?,
    ])
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
        .short_string_bytes(value)
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
        .short_string_bytes(value)
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
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::closure_index(index)
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

    fn function(name: &str) -> crate::NativeStdFunction {
        native_functions(StdLib::Package)
            .iter()
            .find(|function| function.descriptor().name() == name)
            .expect("package native function should exist")
            .function()
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
