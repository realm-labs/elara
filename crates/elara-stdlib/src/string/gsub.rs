//! `string.gsub` native implementation.

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{optional_integer_arg, string_arg};

pub(super) fn string_gsub(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let subject = string_arg(
        runtime,
        *args
            .first()
            .ok_or(NativeErrorKind::MissingArgument { index: 1 })?,
        1,
    )?
    .to_vec();
    let pattern = string_arg(
        runtime,
        *args
            .get(1)
            .ok_or(NativeErrorKind::MissingArgument { index: 2 })?,
        2,
    )?
    .to_vec();
    let replacement = string_arg(
        runtime,
        *args
            .get(2)
            .ok_or(NativeErrorKind::MissingArgument { index: 3 })?,
        3,
    )?
    .to_vec();
    let max = optional_integer_arg(
        args,
        4,
        i64::try_from(subject.len())
            .expect("runtime string length fits LuaInteger")
            .saturating_add(1),
    )?;

    if has_pattern_special(&pattern) {
        return Err(NativeErrorKind::RuntimeError {
            message: "string pattern matching is not supported yet".into(),
        }
        .into());
    }
    if max <= 0 {
        return Ok(vec![
            runtime.intern_short_string(&subject)?,
            Value::integer(0),
        ]);
    }

    let mut output = Vec::new();
    let mut cursor = 0;
    let mut replacements = 0_i64;
    while replacements < max {
        let Some(offset) = plain_find(&subject[cursor..], &pattern) else {
            break;
        };
        let start = cursor + offset;
        output.extend_from_slice(&subject[cursor..start]);
        output.extend_from_slice(&replacement);
        replacements += 1;
        cursor = if pattern.is_empty() {
            start.saturating_add(1)
        } else {
            start + pattern.len()
        };
        if pattern.is_empty() && start < subject.len() {
            output.push(subject[start]);
        }
        if cursor > subject.len() {
            cursor = subject.len();
            break;
        }
    }
    output.extend_from_slice(&subject[cursor..]);

    let result = if replacements == 0 {
        runtime.intern_short_string(&subject)?
    } else {
        runtime.intern_short_string(&output)?
    };
    Ok(vec![result, Value::integer(replacements)])
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

    use super::string_gsub;
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
    fn string_gsub_replaces_literal_matches() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abab");
        let pattern = runtime.push_string(b"a");
        let replacement = runtime.push_string(b"x");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"xbxb".as_slice())
        );
        assert_eq!(values[1], Value::integer(2));
    }

    #[test]
    fn string_gsub_honors_max_replacements() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"aaaa");
        let pattern = runtime.push_string(b"a");
        let replacement = runtime.push_string(b"x");

        let values = string_gsub(
            &mut runtime,
            &[subject, pattern, replacement, Value::integer(2)],
        )
        .expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"xxaa".as_slice())
        );
        assert_eq!(values[1], Value::integer(2));
    }

    #[test]
    fn string_gsub_returns_original_string_and_zero_count_without_match() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"z");
        let replacement = runtime.push_string(b"x");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"abc".as_slice())
        );
        assert_eq!(values[1], Value::integer(0));
    }

    #[test]
    fn string_gsub_reports_pattern_gap_for_magic_patterns() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"a.");
        let replacement = runtime.push_string(b"x");

        assert_eq!(
            string_gsub(&mut runtime, &[subject, pattern, replacement])
                .expect_err("pattern matching should be explicit gap")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "string pattern matching is not supported yet".into()
            }
        );
    }
}
