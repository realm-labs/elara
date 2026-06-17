//! `string.gmatch` native implementation.

use elara_core::Value;

use crate::{NativeError, NativeErrorKind, NativeRuntime, StdLib};

use super::{
    optional_integer_arg,
    pattern::{
        PatternCapture, simple_pattern_match_from_without_start_anchor,
        unsupported_pattern_error_with_captures,
    },
    relative_start, string_arg,
};

const SUBJECT_KEY: Value = Value::integer(1);
const PATTERN_KEY: Value = Value::integer(2);
const CURSOR_KEY: Value = Value::integer(3);
const CALL_KEY_BYTES: &[u8] = b"__call";

pub(super) fn string_gmatch(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let subject_value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let subject = string_arg(runtime, subject_value, 1)?;
    let pattern_value = *args
        .get(1)
        .ok_or(NativeErrorKind::MissingArgument { index: 2 })?;
    let pattern = string_arg(runtime, pattern_value, 2)?;
    let init = relative_start(optional_integer_arg(args, 3, 1)?, subject.len());

    if let Some(error) = unsupported_pattern_error_with_captures(&pattern) {
        return Err(error);
    }

    let cursor = if init > subject.len() {
        subject.len().saturating_add(1)
    } else {
        init - 1
    };
    let cursor =
        i64::try_from(cursor).expect("runtime string cursor position must fit in LuaInteger");
    let state = runtime.create_table(&[
        (SUBJECT_KEY, subject_value),
        (PATTERN_KEY, pattern_value),
        (CURSOR_KEY, Value::integer(cursor)),
    ])?;
    let call_key = runtime.intern_short_string(CALL_KEY_BYTES)?;
    let aux = runtime.native_function(StdLib::String, "__gmatch_aux")?;
    let metatable = runtime.create_table(&[(call_key, aux)])?;
    runtime.table_set_metatable(state, metatable)?;
    Ok(vec![state])
}

pub(super) fn string_gmatch_aux(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let state = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if !state.is_table() {
        return Err(NativeErrorKind::TypeError {
            index: 1,
            expected: "table",
        }
        .into());
    }

    let subject_value = runtime.table_get(state, SUBJECT_KEY)?;
    let pattern_value = runtime.table_get(state, PATTERN_KEY)?;
    let cursor =
        runtime
            .table_get(state, CURSOR_KEY)?
            .as_integer()
            .ok_or(NativeErrorKind::TypeError {
                index: 1,
                expected: "gmatch state",
            })?;
    let cursor =
        usize::try_from(cursor).map_err(|_| NativeErrorKind::ArgumentOutOfRange { index: 1 })?;
    let subject = string_arg(runtime, subject_value, 1)?.to_vec();
    let pattern = string_arg(runtime, pattern_value, 2)?.to_vec();
    if cursor > subject.len() {
        return Ok(Vec::new());
    }

    let Some(match_) = simple_pattern_match_from_without_start_anchor(&subject, &pattern, cursor)
    else {
        return Ok(Vec::new());
    };
    let next_cursor = if match_.start == match_.end {
        match_.end.saturating_add(1)
    } else {
        match_.end
    };
    let next_cursor =
        i64::try_from(next_cursor).expect("runtime string cursor position must fit in LuaInteger");
    runtime.table_set(state, CURSOR_KEY, Value::integer(next_cursor))?;
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

    use super::{string_gmatch, string_gmatch_aux};
    use crate::{NativeError, NativeErrorKind, NativeRuntime, StdLib};

    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        tables: Vec<Vec<(Value, Value)>>,
        metatables: Vec<Option<Value>>,
        gmatch_aux: Value,
    }

    impl Default for TestRuntime {
        fn default() -> Self {
            Self {
                strings: Vec::new(),
                tables: Vec::new(),
                metatables: Vec::new(),
                gmatch_aux: Value::nil(),
            }
        }
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
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            Ok(self.push_string(bytes))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_closure_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }

        fn create_table(&mut self, entries: &[(Value, Value)]) -> Result<Value, NativeError> {
            let index = u32::try_from(self.tables.len()).expect("test table index fits in u32");
            self.tables.push(entries.to_vec());
            self.metatables.push(None);
            Ok(Value::table_index(index))
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

        fn table_set(&mut self, table: Value, key: Value, value: Value) -> Result<(), NativeError> {
            let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
            let table = self
                .tables
                .get_mut(table_index)
                .ok_or_else(non_table_error)?;
            table.push((key, value));
            Ok(())
        }

        fn table_metatable(&self, table: Value) -> Result<Value, NativeError> {
            let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
            self.tables.get(table_index).ok_or_else(non_table_error)?;
            Ok(self
                .metatables
                .get(table_index)
                .copied()
                .flatten()
                .unwrap_or_else(Value::nil))
        }

        fn table_set_metatable(
            &mut self,
            table: Value,
            metatable: Value,
        ) -> Result<(), NativeError> {
            let table_index = table.as_table_index().ok_or_else(non_table_error)? as usize;
            self.tables.get(table_index).ok_or_else(non_table_error)?;
            if !metatable.is_nil() {
                let metatable_index =
                    metatable.as_table_index().ok_or_else(non_table_error)? as usize;
                self.tables
                    .get(metatable_index)
                    .ok_or_else(non_table_error)?;
            }
            self.metatables[table_index] = (!metatable.is_nil()).then_some(metatable);
            Ok(())
        }

        fn native_function(&self, library: StdLib, name: &str) -> Result<Value, NativeError> {
            match (library, name) {
                (StdLib::String, "__gmatch_aux") => Ok(self.gmatch_aux),
                _ => Err(NativeErrorKind::RuntimeError {
                    message: "unknown native helper".into(),
                }
                .into()),
            }
        }
    }

    fn non_table_error() -> NativeError {
        NativeErrorKind::RuntimeError {
            message: "attempt to index a non-table value".into(),
        }
        .into()
    }

    #[test]
    fn string_gmatch_returns_callable_iterator_state() {
        let mut runtime = TestRuntime {
            gmatch_aux: Value::native_function_index(7),
            ..TestRuntime::default()
        };
        let subject = runtime.push_string(b"a1 b22");
        let pattern = runtime.push_string(b"%d+");

        let values = string_gmatch(&mut runtime, &[subject, pattern]).expect("gmatch should pass");

        assert_eq!(values.len(), 1);
        assert!(values[0].is_table());
        let metatable = runtime
            .table_metatable(values[0])
            .expect("gmatch state should have a metatable");
        let call_key = runtime.push_string(b"__call");
        assert_eq!(
            runtime
                .table_get(metatable, call_key)
                .expect("metatable __call should be readable"),
            Value::native_function_index(7)
        );
    }

    #[test]
    fn string_gmatch_aux_iterates_matches() {
        let mut runtime = TestRuntime {
            gmatch_aux: Value::native_function_index(7),
            ..TestRuntime::default()
        };
        let subject = runtime.push_string(b"a1 b22");
        let pattern = runtime.push_string(b"%d+");
        let values = string_gmatch(&mut runtime, &[subject, pattern]).expect("gmatch should pass");
        let state = values[0];

        let first = string_gmatch_aux(&mut runtime, &[state, Value::nil()])
            .expect("first gmatch aux should pass");
        let second = string_gmatch_aux(&mut runtime, &[state, first[0]])
            .expect("second gmatch aux should pass");
        let end = string_gmatch_aux(&mut runtime, &[state, second[0]])
            .expect("third gmatch aux should pass");

        assert_eq!(runtime.short_string_bytes(first[0]), Some(b"1".as_slice()));
        assert_eq!(
            runtime.short_string_bytes(second[0]),
            Some(b"22".as_slice())
        );
        assert_eq!(end, Vec::<Value>::new());
    }

    #[test]
    fn string_gmatch_aux_returns_captures_when_present() {
        let mut runtime = TestRuntime {
            gmatch_aux: Value::native_function_index(7),
            ..TestRuntime::default()
        };
        let subject = runtime.push_string(b"a1 b22");
        let pattern = runtime.push_string(b"(%a)(%d+)");
        let values = string_gmatch(&mut runtime, &[subject, pattern]).expect("gmatch should pass");
        let state = values[0];

        let first = string_gmatch_aux(&mut runtime, &[state, Value::nil()])
            .expect("first gmatch aux should pass");

        assert_eq!(first.len(), 2);
        assert_eq!(runtime.short_string_bytes(first[0]), Some(b"a".as_slice()));
        assert_eq!(runtime.short_string_bytes(first[1]), Some(b"1".as_slice()));
    }

    #[test]
    fn string_gmatch_aux_returns_position_captures() {
        let mut runtime = TestRuntime {
            gmatch_aux: Value::native_function_index(7),
            ..TestRuntime::default()
        };
        let subject = runtime.push_string(b"abc");
        let pattern = runtime.push_string(b"()");
        let values = string_gmatch(&mut runtime, &[subject, pattern]).expect("gmatch should pass");
        let state = values[0];

        let first = string_gmatch_aux(&mut runtime, &[state, Value::nil()])
            .expect("first gmatch aux should pass");
        let second = string_gmatch_aux(&mut runtime, &[state, first[0]])
            .expect("second gmatch aux should pass");

        assert_eq!(first, vec![Value::integer(1)]);
        assert_eq!(second, vec![Value::integer(2)]);
    }

    #[test]
    fn string_gmatch_treats_start_anchor_as_literal_pattern_byte() {
        let mut runtime = TestRuntime {
            gmatch_aux: Value::native_function_index(7),
            ..TestRuntime::default()
        };
        let subject = runtime.push_string(b"a^b ^c");
        let pattern = runtime.push_string(b"^.");
        let values = string_gmatch(&mut runtime, &[subject, pattern]).expect("gmatch should pass");
        let state = values[0];

        let first = string_gmatch_aux(&mut runtime, &[state, Value::nil()])
            .expect("first gmatch aux should pass");
        let second = string_gmatch_aux(&mut runtime, &[state, first[0]])
            .expect("second gmatch aux should pass");

        assert_eq!(runtime.short_string_bytes(first[0]), Some(b"^b".as_slice()));
        assert_eq!(
            runtime.short_string_bytes(second[0]),
            Some(b"^c".as_slice())
        );
    }

    #[test]
    fn string_gmatch_aux_advances_empty_matches() {
        let mut runtime = TestRuntime {
            gmatch_aux: Value::native_function_index(7),
            ..TestRuntime::default()
        };
        let subject = runtime.push_string(b"ab");
        let pattern = runtime.push_string(b"");
        let values = string_gmatch(&mut runtime, &[subject, pattern]).expect("gmatch should pass");
        let state = values[0];

        let first = string_gmatch_aux(&mut runtime, &[state, Value::nil()])
            .expect("first gmatch aux should pass");
        let second = string_gmatch_aux(&mut runtime, &[state, first[0]])
            .expect("second gmatch aux should pass");
        let third = string_gmatch_aux(&mut runtime, &[state, second[0]])
            .expect("third gmatch aux should pass");
        let end = string_gmatch_aux(&mut runtime, &[state, third[0]])
            .expect("fourth gmatch aux should pass");

        assert_eq!(runtime.short_string_bytes(first[0]), Some(b"".as_slice()));
        assert_eq!(runtime.short_string_bytes(second[0]), Some(b"".as_slice()));
        assert_eq!(runtime.short_string_bytes(third[0]), Some(b"".as_slice()));
        assert_eq!(end, Vec::<Value>::new());
    }
}
