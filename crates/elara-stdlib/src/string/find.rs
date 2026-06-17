//! `string.find` native implementation.

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{
    optional_integer_arg,
    pattern::{PatternCapture, simple_pattern_match_from, unsupported_pattern_error_with_captures},
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
    if !plain && let Some(error) = unsupported_pattern_error_with_captures(&pattern) {
        return Err(error);
    }

    let offset = init - 1;
    if plain {
        return Ok(plain_find(&subject[offset..], &pattern).map_or_else(
            || vec![Value::nil()],
            |start| {
                let start = offset + start;
                vec![
                    Value::integer(i64::try_from(start + 1).expect("string index fits LuaInteger")),
                    Value::integer(
                        i64::try_from(start + pattern.len()).expect("string index fits LuaInteger"),
                    ),
                ]
            },
        ));
    }

    let Some(match_) = simple_pattern_match_from(&subject, &pattern, offset) else {
        return Ok(vec![Value::nil()]);
    };
    let mut values = vec![
        Value::integer(i64::try_from(match_.start + 1).expect("string index fits LuaInteger")),
        Value::integer(i64::try_from(match_.end).expect("string index fits LuaInteger")),
    ];
    if match_.captures.is_empty() {
        return Ok(values);
    }

    let subject = subject.to_vec();
    for capture in match_.captures {
        values.push(capture_value(runtime, &subject, capture)?);
    }
    Ok(values)
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
    fn string_find_matches_start_and_end_anchors() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let start_pattern = runtime.push_string(b"^a.");
        let end_pattern = runtime.push_string(b"b.$");
        let absent_pattern = runtime.push_string(b"^b");

        assert_eq!(
            string_find(&mut runtime, &[subject, start_pattern]).expect("find should pass"),
            vec![Value::integer(1), Value::integer(2)]
        );
        assert_eq!(
            string_find(&mut runtime, &[subject, end_pattern]).expect("find should pass"),
            vec![Value::integer(5), Value::integer(6)]
        );
        assert_eq!(
            string_find(&mut runtime, &[subject, absent_pattern]).expect("find should pass"),
            vec![Value::nil()]
        );
    }

    #[test]
    fn string_find_start_anchor_honors_init_position() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcabc");
        let pattern = runtime.push_string(b"^b.");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern, Value::integer(2)])
                .expect("find should pass"),
            vec![Value::integer(2), Value::integer(3)]
        );
    }

    #[test]
    fn string_find_matches_percent_classes_and_escaped_literals() {
        let mut runtime = TestRuntime::default();
        let class_subject = runtime.push_string(b"abc123");
        let class_pattern = runtime.push_string(b"%d%d");
        let literal_subject = runtime.push_string(b"a+b");
        let literal_pattern = runtime.push_string(b"a%+");

        assert_eq!(
            string_find(&mut runtime, &[class_subject, class_pattern]).expect("find should pass"),
            vec![Value::integer(4), Value::integer(5)]
        );
        assert_eq!(
            string_find(&mut runtime, &[literal_subject, literal_pattern])
                .expect("find should pass"),
            vec![Value::integer(1), Value::integer(2)]
        );
    }

    #[test]
    fn string_find_matches_bracket_classes() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc123");
        let range_pattern = runtime.push_string(b"[0-9][0-9]");
        let negated_pattern = runtime.push_string(b"[^a-c][0-9]");

        assert_eq!(
            string_find(&mut runtime, &[subject, range_pattern]).expect("find should pass"),
            vec![Value::integer(4), Value::integer(5)]
        );
        assert_eq!(
            string_find(&mut runtime, &[subject, negated_pattern]).expect("find should pass"),
            vec![Value::integer(4), Value::integer(5)]
        );
    }

    #[test]
    fn string_find_matches_quantifiers() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"aaab");
        let greedy_pattern = runtime.push_string(b"a+b");
        let optional_pattern = runtime.push_string(b"ac?b");

        assert_eq!(
            string_find(&mut runtime, &[subject, greedy_pattern]).expect("find should pass"),
            vec![Value::integer(1), Value::integer(4)]
        );
        assert_eq!(
            string_find(&mut runtime, &[subject, optional_pattern]).expect("find should pass"),
            vec![Value::integer(3), Value::integer(4)]
        );
    }

    #[test]
    fn string_find_returns_captures_after_bounds() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc123");
        let pattern = runtime.push_string(b"(%a+)(%d+)");

        let values = string_find(&mut runtime, &[subject, pattern]).expect("find should pass");

        assert_eq!(values[0], Value::integer(1));
        assert_eq!(values[1], Value::integer(6));
        assert_eq!(
            runtime.short_string_bytes(values[2]),
            Some(b"abc".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(values[3]),
            Some(b"123".as_slice())
        );
    }

    #[test]
    fn string_find_matches_balanced_delimiters() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"a(b(c)d)e");
        let pattern = runtime.push_string(b"%b()");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern]).expect("find should pass"),
            vec![Value::integer(2), Value::integer(8)]
        );
    }

    #[test]
    fn string_find_matches_frontiers() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc 123");
        let pattern = runtime.push_string(b"%f[%d]%d+");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern]).expect("find should pass"),
            vec![Value::integer(5), Value::integer(7)]
        );
    }

    #[test]
    fn string_find_matches_capture_backreferences() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"alo alo");
        let pattern = runtime.push_string(b"(%a+) %1");

        let values = string_find(&mut runtime, &[subject, pattern]).expect("find should pass");

        assert_eq!(values[0], Value::integer(1));
        assert_eq!(values[1], Value::integer(7));
        assert_eq!(
            runtime.short_string_bytes(values[2]),
            Some(b"alo".as_slice())
        );
    }

    #[test]
    fn string_find_returns_position_captures() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"flaaap");
        let pattern = runtime.push_string(b"()aa()");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern]).expect("find should pass"),
            vec![
                Value::integer(3),
                Value::integer(4),
                Value::integer(3),
                Value::integer(5)
            ]
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
    fn string_find_reports_invalid_capture_patterns() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"%0");
        let out_of_range = runtime.push_string(b"(%a+)%2");

        assert_eq!(
            string_find(&mut runtime, &[subject, pattern])
                .expect_err("invalid capture should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "invalid capture index %0".into()
            }
        );
        assert_eq!(
            string_find(&mut runtime, &[subject, out_of_range])
                .expect_err("out-of-range capture should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "invalid capture index %2".into()
            }
        );
    }

    #[test]
    fn string_find_reports_malformed_patterns() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc");
        let trailing_escape = runtime.push_string(b"%");
        let unfinished_capture = runtime.push_string(b"(a");
        let invalid_capture = runtime.push_string(b"%a+)");
        let empty_bracket_class = runtime.push_string(b"[]");
        let empty_frontier_class = runtime.push_string(b"%f[]");
        let too_many_capture_pattern = "()".repeat(33);
        let too_many_captures = runtime.push_string(too_many_capture_pattern.as_bytes());

        assert_eq!(
            string_find(&mut runtime, &[subject, trailing_escape])
                .expect_err("trailing escape should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "malformed pattern (ends with '%')".into()
            }
        );
        assert_eq!(
            string_find(&mut runtime, &[subject, unfinished_capture])
                .expect_err("unfinished capture should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "unfinished capture".into()
            }
        );
        assert_eq!(
            string_find(&mut runtime, &[subject, invalid_capture])
                .expect_err("invalid capture should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "invalid pattern capture".into()
            }
        );
        for pattern in [empty_bracket_class, empty_frontier_class] {
            assert_eq!(
                string_find(&mut runtime, &[subject, pattern])
                    .expect_err("empty bracket class should fail")
                    .kind(),
                &NativeErrorKind::RuntimeError {
                    message: "malformed pattern (missing ']')".into()
                }
            );
        }
        assert_eq!(
            string_find(&mut runtime, &[subject, too_many_captures])
                .expect_err("too many captures should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "too many captures".into()
            }
        );
    }
}
