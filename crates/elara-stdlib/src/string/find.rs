//! `string.find` native implementation.

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{
    optional_integer_arg,
    pattern::{has_unsupported_pattern_special, simple_pattern_find},
    relative_start, string_arg,
};

pub(super) fn string_find(
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

    let plain = args.get(3).is_some_and(|value| is_truthy(*value));
    if !plain && has_unsupported_pattern_special(pattern) {
        return Err(NativeErrorKind::RuntimeError {
            message: "string pattern matching is not supported yet".into(),
        }
        .into());
    }

    let offset = init - 1;
    let found = if plain {
        plain_find(&subject[offset..], pattern).map(|start| (start, start + pattern.len()))
    } else {
        simple_pattern_find(&subject[offset..], pattern)
    };
    Ok(found.map_or_else(
        || vec![Value::nil()],
        |(start, end)| {
            let start = offset + start;
            let end = offset + end;
            vec![
                Value::integer(i64::try_from(start + 1).expect("string index fits LuaInteger")),
                Value::integer(i64::try_from(end).expect("string index fits LuaInteger")),
            ]
        },
    ))
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

fn plain_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::string_find;
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
    fn string_find_returns_plain_match_bounds() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let pattern = runtime.push_string(b"ca");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern]).expect("find should pass"),
            vec![Value::integer(3), Value::integer(4)]
        );
    }

    #[test]
    fn string_find_honors_init_and_plain_flag() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"a.c a.c");
        let pattern = runtime.push_string(b"a.c");

        assert_eq!(
            string_find(
                &mut runtime,
                &[subject, pattern, Value::integer(2), Value::boolean(true)]
            )
            .expect("find should pass"),
            vec![Value::integer(5), Value::integer(7)]
        );
    }

    #[test]
    fn string_find_matches_dot_wildcard() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let pattern = runtime.push_string(b"a.");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern]).expect("find should pass"),
            vec![Value::integer(1), Value::integer(2)]
        );
    }

    #[test]
    fn string_find_returns_nil_when_plain_match_is_absent() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"z");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern]).expect("find should pass"),
            vec![Value::nil()]
        );
    }

    #[test]
    fn string_find_reports_pattern_gap_for_magic_patterns() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"a+");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern])
                .expect_err("pattern matching should be explicit gap")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "string pattern matching is not supported yet".into()
            }
        );
    }
}
