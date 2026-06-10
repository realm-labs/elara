//! `string.match` native implementation.

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{optional_integer_arg, relative_start, string_arg};

pub(super) fn string_match(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let subject = string_arg(
        runtime,
        *args
            .first()
            .ok_or(NativeErrorKind::MissingArgument { index: 1 })?,
        1,
    )?;
    let pattern = string_arg(
        runtime,
        *args
            .get(1)
            .ok_or(NativeErrorKind::MissingArgument { index: 2 })?,
        2,
    )?;
    let init = relative_start(optional_integer_arg(args, 3, 1)?, subject.len());
    if init > subject.len() {
        return Ok(vec![Value::nil()]);
    }
    if has_pattern_special(pattern) {
        return Err(NativeErrorKind::RuntimeError {
            message: "string pattern matching is not supported yet".into(),
        }
        .into());
    }

    let offset = init - 1;
    let Some(start) = plain_find(&subject[offset..], pattern) else {
        return Ok(vec![Value::nil()]);
    };
    let start = offset + start;
    let end = start + pattern.len();
    let matched = subject[start..end].to_vec();
    Ok(vec![runtime.intern_short_string(&matched)?])
}

fn plain_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn has_pattern_special(pattern: &[u8]) -> bool {
    pattern.iter().any(|byte| {
        matches!(
            byte,
            b'^' | b'$' | b'*' | b'+' | b'?' | b'.' | b'(' | b'[' | b'%' | b'-'
        )
    })
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::string_match;
    use crate::{NativeError, NativeErrorKind, NativeRuntime};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::table_index(index)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_table_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }
    }

    #[test]
    fn string_match_returns_literal_match() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let pattern = runtime.push_string(b"ca");

        let values = string_match(&mut runtime, &[subject, pattern]).expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"ca".as_slice())
        );
    }

    #[test]
    fn string_match_honors_init() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let pattern = runtime.push_string(b"ab");

        let values = string_match(&mut runtime, &[subject, pattern, Value::integer(2)])
            .expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"ab".as_slice())
        );
    }

    #[test]
    fn string_match_returns_nil_when_literal_match_is_absent() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"z");

        assert_eq!(
            string_match(&mut runtime, &[subject, pattern]).expect("match should pass"),
            vec![Value::nil()]
        );
    }

    #[test]
    fn string_match_reports_pattern_gap_for_magic_patterns() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"a.");

        assert_eq!(
            string_match(&mut runtime, &[subject, pattern])
                .expect_err("pattern matching should be explicit gap")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "string pattern matching is not supported yet".into()
            }
        );
    }
}
