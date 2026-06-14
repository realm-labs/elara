use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

const UNSUPPORTED_MESSAGE: &[u8] = b"dynamic library loading is not supported by this runtime";
const OPEN_STAGE: &[u8] = b"open";

pub(super) fn package_loadlib(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    string_arg(runtime, args, 1)?;
    string_arg(runtime, args, 2)?;

    Ok(vec![
        Value::nil(),
        runtime.intern_string(UNSUPPORTED_MESSAGE)?,
        runtime.intern_short_string(OPEN_STAGE)?,
    ])
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
    use elara_core::Value;

    use crate::{NativeError, NativeErrorKind, NativeRuntime};

    use super::package_loadlib;

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

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            self.bytes(value)
        }
    }

    #[test]
    fn package_loadlib_reports_unsupported_loader() {
        let mut runtime = TestRuntime::default();
        let library = runtime.push_string(b"missing.so");
        let init = runtime.push_string(b"luaopen_missing");

        let result = package_loadlib(&mut runtime, &[library, init]).expect("loadlib should pass");

        assert_eq!(result[0], Value::nil());
        assert_eq!(
            runtime.bytes(result[1]),
            Some(b"dynamic library loading is not supported by this runtime".as_slice())
        );
        assert_eq!(runtime.bytes(result[2]), Some(b"open".as_slice()));
    }

    #[test]
    fn package_loadlib_validates_arguments() {
        let mut runtime = TestRuntime::default();
        let library = runtime.push_string(b"missing.so");

        assert_eq!(
            package_loadlib(&mut runtime, &[])
                .expect_err("missing library")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 1 }
        );
        assert_eq!(
            package_loadlib(&mut runtime, &[library])
                .expect_err("missing function")
                .kind(),
            &NativeErrorKind::MissingArgument { index: 2 }
        );
        assert_eq!(
            package_loadlib(&mut runtime, &[Value::integer(1), library])
                .expect_err("non-string library")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "string",
            }
        );
    }
}
