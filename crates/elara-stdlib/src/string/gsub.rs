//! `string.gsub` native implementation.

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime};

use super::{
    optional_integer_arg,
    pattern::{
        PatternMatch, has_unsupported_pattern_special_with_captures, is_start_anchored,
        simple_pattern_match_from,
    },
    string_arg,
};

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
    let replacement = replacement_arg(
        runtime,
        *args
            .get(2)
            .ok_or(NativeErrorKind::MissingArgument { index: 3 })?,
    )?;
    let max = optional_integer_arg(
        args,
        4,
        i64::try_from(subject.len())
            .expect("runtime string length fits LuaInteger")
            .saturating_add(1),
    )?;

    if has_unsupported_pattern_special_with_captures(&pattern) {
        return Err(NativeErrorKind::RuntimeError {
            message: "string pattern matching is not supported yet".into(),
        }
        .into());
    }
    let anchored = is_start_anchored(&pattern);
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
        let Some(match_) = simple_pattern_match_from(&subject, &pattern, cursor) else {
            break;
        };
        output.extend_from_slice(&subject[cursor..match_.start]);
        append_replacement(runtime, &mut output, &subject, &replacement, &match_)?;
        replacements += 1;
        let matched_empty = match_.start == match_.end;
        cursor = if matched_empty {
            match_.start.saturating_add(1)
        } else {
            match_.end
        };
        if matched_empty && match_.start < subject.len() {
            output.push(subject[match_.start]);
        }
        if cursor > subject.len() {
            cursor = subject.len();
            break;
        }
        if anchored {
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

enum Replacement {
    String(Vec<u8>),
    Table(Value),
    Function(Value),
}

fn replacement_arg(runtime: &dyn NativeRuntime, value: Value) -> Result<Replacement, NativeError> {
    if let Some(bytes) = runtime.short_string_bytes(value) {
        return Ok(Replacement::String(bytes.to_vec()));
    }
    if value.is_table() {
        return Ok(Replacement::Table(value));
    }
    if value.is_closure() {
        return Ok(Replacement::Function(value));
    }
    Err(NativeErrorKind::TypeError {
        index: 3,
        expected: "string, table, or function",
    }
    .into())
}

fn append_replacement(
    runtime: &mut dyn NativeRuntime,
    output: &mut Vec<u8>,
    subject: &[u8],
    replacement: &Replacement,
    match_: &PatternMatch,
) -> Result<(), NativeError> {
    match replacement {
        Replacement::String(replacement) => {
            append_string_replacement(output, subject, replacement, match_)
        }
        Replacement::Table(table) => {
            append_table_replacement(runtime, output, subject, *table, match_)
        }
        Replacement::Function(function) => {
            append_function_replacement(runtime, output, subject, *function, match_)
        }
    }
}

fn append_string_replacement(
    output: &mut Vec<u8>,
    subject: &[u8],
    replacement: &[u8],
    match_: &PatternMatch,
) -> Result<(), NativeError> {
    let mut index = 0;
    while let Some(byte) = replacement.get(index).copied() {
        if byte != b'%' {
            output.push(byte);
            index += 1;
            continue;
        }
        let Some(next) = replacement.get(index + 1).copied() else {
            output.push(byte);
            index += 1;
            continue;
        };
        match next {
            b'%' => output.push(b'%'),
            b'0' => output.extend_from_slice(&subject[match_.start..match_.end]),
            b'1'..=b'9' => {
                let capture_index = usize::from(next - b'1');
                let Some((start, end)) = match_.captures.get(capture_index).copied() else {
                    return Err(NativeErrorKind::ArgumentOutOfRange { index: 3 }.into());
                };
                output.extend_from_slice(&subject[start..end]);
            }
            _ => output.push(next),
        }
        index += 2;
    }
    Ok(())
}

fn append_table_replacement(
    runtime: &mut dyn NativeRuntime,
    output: &mut Vec<u8>,
    subject: &[u8],
    table: Value,
    match_: &PatternMatch,
) -> Result<(), NativeError> {
    let key = replacement_args(runtime, subject, match_)?
        .into_iter()
        .next()
        .unwrap_or_else(Value::nil);
    let value = runtime.table_get(table, key)?;
    append_replacement_value(runtime, output, subject, match_, value)
}

fn append_function_replacement(
    runtime: &mut dyn NativeRuntime,
    output: &mut Vec<u8>,
    subject: &[u8],
    function: Value,
    match_: &PatternMatch,
) -> Result<(), NativeError> {
    let args = replacement_args(runtime, subject, match_)?;
    let values = runtime
        .protected_call(function, &args)?
        .map_err(NativeError::lua_error)?;
    let value = values.first().copied().unwrap_or_else(Value::nil);
    append_replacement_value(runtime, output, subject, match_, value)
}

fn replacement_args(
    runtime: &mut dyn NativeRuntime,
    subject: &[u8],
    match_: &PatternMatch,
) -> Result<Vec<Value>, NativeError> {
    if match_.captures.is_empty() {
        return Ok(vec![
            runtime.intern_short_string(&subject[match_.start..match_.end])?,
        ]);
    }
    match_
        .captures
        .iter()
        .copied()
        .map(|(start, end)| runtime.intern_short_string(&subject[start..end]))
        .collect()
}

fn append_replacement_value(
    runtime: &dyn NativeRuntime,
    output: &mut Vec<u8>,
    subject: &[u8],
    match_: &PatternMatch,
    value: Value,
) -> Result<(), NativeError> {
    if value.is_nil() || value.as_bool() == Some(false) {
        output.extend_from_slice(&subject[match_.start..match_.end]);
        return Ok(());
    }
    if let Some(bytes) = runtime.short_string_bytes(value) {
        output.extend_from_slice(bytes);
        return Ok(());
    }
    if let Some(integer) = value.as_integer() {
        output.extend_from_slice(integer.to_string().as_bytes());
        return Ok(());
    }
    if let Some(float) = value.as_float() {
        output.extend_from_slice(float.to_string().as_bytes());
        return Ok(());
    }
    Err(NativeErrorKind::TypeError {
        index: 3,
        expected: "string or number replacement",
    }
    .into())
}

#[cfg(test)]
mod tests {
    use elara_core::Value;

    use super::string_gsub;
    use crate::{NativeError, NativeErrorKind, NativeRuntime};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        tables: Vec<Vec<(Value, Value)>>,
        protected_results: Vec<Result<Vec<Value>, Box<str>>>,
    }

    impl TestRuntime {
        fn push_string(&mut self, bytes: &[u8]) -> Value {
            if let Some(index) = self
                .strings
                .iter()
                .position(|string| string.as_ref() == bytes)
            {
                let index = u32::try_from(index).expect("test string index fits in u32");
                return Value::closure_index(index);
            }
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Value::closure_index(index)
        }

        fn push_table(&mut self, entries: Vec<(Value, Value)>) -> Value {
            let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
            self.tables.push(entries);
            Value::table_index(index)
        }
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }

        fn table_get(&self, table: Value, key: Value) -> Result<Value, NativeError> {
            let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
            let table = self.tables.get(table_index).ok_or_else(non_table_error)?;
            Ok(table
                .iter()
                .rev()
                .find_map(|(entry_key, value)| (*entry_key == key).then_some(*value))
                .unwrap_or_else(Value::nil))
        }

        fn protected_call(
            &mut self,
            _function: Value,
            _args: &[Value],
        ) -> Result<Result<Vec<Value>, Box<str>>, NativeError> {
            if self.protected_results.is_empty() {
                return Ok(Err("missing protected result".into()));
            }
            Ok(self.protected_results.remove(0))
        }
    }

    fn non_table_error() -> NativeError {
        NativeErrorKind::RuntimeError {
            message: "attempt to index a non-table value".into(),
        }
        .into()
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
    fn string_gsub_replaces_dot_wildcard_matches() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abcadc");
        let pattern = runtime.push_string(b"a.");
        let replacement = runtime.push_string(b"x");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"xcxc".as_slice())
        );
        assert_eq!(values[1], Value::integer(2));
    }

    #[test]
    fn string_gsub_honors_start_and_end_anchors() {
        let mut runtime = TestRuntime::default();
        let start_subject = runtime.push_string(b"abc");
        let start_pattern = runtime.push_string(b"^");
        let end_subject = runtime.push_string(b"abc");
        let end_pattern = runtime.push_string(b"$");
        let prefix_subject = runtime.push_string(b"abcabc");
        let prefix_pattern = runtime.push_string(b"^a.");
        let suffix_subject = runtime.push_string(b"abcabc");
        let suffix_pattern = runtime.push_string(b"b.$");
        let replacement = runtime.push_string(b"x");

        let start_values = string_gsub(&mut runtime, &[start_subject, start_pattern, replacement])
            .expect("start anchor gsub should pass");
        let end_values = string_gsub(&mut runtime, &[end_subject, end_pattern, replacement])
            .expect("end anchor gsub should pass");
        let prefix_values =
            string_gsub(&mut runtime, &[prefix_subject, prefix_pattern, replacement])
                .expect("prefix anchor gsub should pass");
        let suffix_values =
            string_gsub(&mut runtime, &[suffix_subject, suffix_pattern, replacement])
                .expect("suffix anchor gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(start_values[0]),
            Some(b"xabc".as_slice())
        );
        assert_eq!(start_values[1], Value::integer(1));
        assert_eq!(
            runtime.short_string_bytes(end_values[0]),
            Some(b"abcx".as_slice())
        );
        assert_eq!(end_values[1], Value::integer(1));
        assert_eq!(
            runtime.short_string_bytes(prefix_values[0]),
            Some(b"xcabc".as_slice())
        );
        assert_eq!(prefix_values[1], Value::integer(1));
        assert_eq!(
            runtime.short_string_bytes(suffix_values[0]),
            Some(b"abcax".as_slice())
        );
        assert_eq!(suffix_values[1], Value::integer(1));
    }

    #[test]
    fn string_gsub_replaces_percent_class_matches() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"a1b2");
        let pattern = runtime.push_string(b"%d");
        let replacement = runtime.push_string(b"x");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"axbx".as_slice())
        );
        assert_eq!(values[1], Value::integer(2));
    }

    #[test]
    fn string_gsub_replaces_bracket_class_matches() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"a1b2c3");
        let pattern = runtime.push_string(b"[%a][0-9]");
        let replacement = runtime.push_string(b"x");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"xxx".as_slice())
        );
        assert_eq!(values[1], Value::integer(3));
    }

    #[test]
    fn string_gsub_replaces_quantifier_matches() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"aaabbb");
        let pattern = runtime.push_string(b"a+");
        let replacement = runtime.push_string(b"x");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"xbbb".as_slice())
        );
        assert_eq!(values[1], Value::integer(1));
    }

    #[test]
    fn string_gsub_expands_replacement_captures() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc123");
        let pattern = runtime.push_string(b"(%a+)(%d+)");
        let replacement = runtime.push_string(b"%2-%1-%0-%%");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"123-abc-abc123-%".as_slice())
        );
        assert_eq!(values[1], Value::integer(1));
    }

    #[test]
    fn string_gsub_uses_table_replacement_by_capture_key() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc123");
        let pattern = runtime.push_string(b"(%a+)");
        let key = runtime.push_string(b"abc");
        let value = runtime.push_string(b"word");
        let replacement = runtime.push_table(vec![(key, value)]);

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"word123".as_slice())
        );
        assert_eq!(values[1], Value::integer(1));
    }

    #[test]
    fn string_gsub_uses_function_replacement_with_captures() {
        let mut runtime = TestRuntime {
            protected_results: vec![Ok(vec![Value::integer(99)])],
            ..TestRuntime::default()
        };
        let subject = runtime.push_string(b"abc123");
        let pattern = runtime.push_string(b"(%a+)(%d+)");
        let replacement = Value::native_function_index(3);

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"99".as_slice())
        );
        assert_eq!(values[1], Value::integer(1));
    }

    #[test]
    fn string_gsub_replaces_balanced_delimiters() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"a(b(c)d)e");
        let pattern = runtime.push_string(b"%b()");
        let replacement = runtime.push_string(b"x");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"axe".as_slice())
        );
        assert_eq!(values[1], Value::integer(1));
    }

    #[test]
    fn string_gsub_replaces_frontier_matches() {
        let mut runtime = TestRuntime::default();
        let subject = runtime.push_string(b"abc 123 def 45");
        let pattern = runtime.push_string(b"%f[%d]%d+");
        let replacement = runtime.push_string(b"n");

        let values =
            string_gsub(&mut runtime, &[subject, pattern, replacement]).expect("gsub should pass");

        assert_eq!(
            runtime.short_string_bytes(values[0]),
            Some(b"abc n def n".as_slice())
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
        let pattern = runtime.push_string(b"%1");
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
