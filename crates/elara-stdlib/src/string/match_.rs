//! `string.match` native implementation.

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{
    optional_integer_arg,
    pattern::{
        PatternCapture, has_unsupported_pattern_special_with_captures, simple_pattern_match_from,
    },
    relative_start, string_arg,
};

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
    if has_unsupported_pattern_special_with_captures(&pattern) {
        return Err(NativeErrorKind::RuntimeError {
            message: "string pattern matching is not supported yet".into(),
        }
        .into());
    }

    let offset = init - 1;
    let Some(match_) = simple_pattern_match_from(&subject, &pattern, offset) else {
        return Ok(vec![Value::nil()]);
    };
    let subject = subject.to_vec();
    if match_.captures.is_empty() {
        return Ok(vec![
            runtime.intern_string(&subject[match_.start..match_.end])?,
        ]);
    }

    match_
        .captures
        .into_iter()
        .map(|capture| capture_value(runtime, &subject, capture))
        .collect()
}

fn capture_value(
    runtime: &mut dyn NativeRuntime,
    subject: &[u8],
    capture: PatternCapture,
) -> Result<Value, NativeError> {
    match capture {
        PatternCapture::String { start, end } => runtime.intern_string(&subject[start..end]),
        PatternCapture::Position(position) => Ok(Value::integer(
            i64::try_from(position + 1).expect("capture position fits LuaInteger"),
        )),
    }
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
    fn string_match_matches_dot_wildcard() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let pattern = runtime.push_string(b"b.");

        let values = string_match(&mut runtime, &[subject, pattern]).expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"bc".as_slice())
        );
    }

    #[test]
    fn string_match_matches_start_and_end_anchors() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let start_pattern = runtime.push_string(b"^a.");
        let end_pattern = runtime.push_string(b"b.$");

        let start_values =
            string_match(&mut runtime, &[subject, start_pattern]).expect("match should pass");
        let end_values =
            string_match(&mut runtime, &[subject, end_pattern]).expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(start_values[0]),
            Some(b"ab".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(end_values[0]),
            Some(b"bc".as_slice())
        );
    }

    #[test]
    fn string_match_start_anchor_honors_init_position() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let pattern = runtime.push_string(b"^b.");

        let values = string_match(&mut runtime, &[subject, pattern, Value::integer(2)])
            .expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"bc".as_slice())
        );
    }

    #[test]
    fn string_match_matches_percent_classes_and_escaped_literals() {
        let mut runtime = TestRuntime::default();
        let class_subject = runtime.push_string(b"abc123");
        let class_pattern = runtime.push_string(b"%d%d");
        let literal_subject = runtime.push_string(b"a+b");
        let literal_pattern = runtime.push_string(b"a%+");

        let class_values = string_match(&mut runtime, &[class_subject, class_pattern])
            .expect("class match should pass");
        let literal_values = string_match(&mut runtime, &[literal_subject, literal_pattern])
            .expect("literal match should pass");

        assert_eq!(
            runtime.short_string_bytes(class_values[0]),
            Some(b"12".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(literal_values[0]),
            Some(b"a+".as_slice())
        );
    }

    #[test]
    fn string_match_matches_bracket_classes() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc123");
        let pattern = runtime.push_string(b"[%a][0-9]");

        let values = string_match(&mut runtime, &[subject, pattern]).expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"c1".as_slice())
        );
    }

    #[test]
    fn string_match_matches_quantifiers() {
        let mut runtime = TestRuntime::default();
        let greedy_subject = runtime.push_string(b"aaab");
        let greedy_pattern = runtime.push_string(b"a+b");
        let minimal_subject = runtime.push_string(b"abcb");
        let minimal_pattern = runtime.push_string(b"a.-b");

        let greedy_values = string_match(&mut runtime, &[greedy_subject, greedy_pattern])
            .expect("greedy match should pass");
        let minimal_values = string_match(&mut runtime, &[minimal_subject, minimal_pattern])
            .expect("minimal match should pass");

        assert_eq!(
            runtime.short_string_bytes(greedy_values[0]),
            Some(b"aaab".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(minimal_values[0]),
            Some(b"ab".as_slice())
        );
    }

    #[test]
    fn string_match_returns_captures_when_present() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc123");
        let pattern = runtime.push_string(b"(%a+)(%d+)");

        let values = string_match(&mut runtime, &[subject, pattern]).expect("match should pass");

        assert_eq!(values.len(), 2);
        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"abc".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(values[1]),
            Some(b"123".as_slice())
        );
    }

    #[test]
    fn string_match_matches_balanced_delimiters() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"a(b(c)d)e");
        let pattern = runtime.push_string(b"%b()");

        let values = string_match(&mut runtime, &[subject, pattern]).expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"(b(c)d)".as_slice())
        );
    }

    #[test]
    fn string_match_matches_frontiers() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc 123");
        let pattern = runtime.push_string(b"%f[%d]%d+");

        let values = string_match(&mut runtime, &[subject, pattern]).expect("match should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"123".as_slice())
        );
    }

    #[test]
    fn string_match_matches_capture_backreferences() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"alo alo");
        let pattern = runtime.push_string(b"(%a+) %1");

        let values = string_match(&mut runtime, &[subject, pattern]).expect("match should pass");

        assert_eq!(values.len(), 1);
        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"alo".as_slice())
        );
    }

    #[test]
    fn string_match_returns_position_captures() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"flaaap");
        let pattern = runtime.push_string(b"()aa()");

        assert_eq!(
            string_match(&mut runtime, &[subject, pattern]).expect("match should pass"),
            vec![Value::integer(3), Value::integer(5)]
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
        let pattern = runtime.push_string(b"%0");

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
