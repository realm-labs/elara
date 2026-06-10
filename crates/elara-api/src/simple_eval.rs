//! Simple source evaluation path.

use elara_compiler::compile_simple_chunk;
use elara_core::{Diagnostic, SourceId, Value};
use elara_interp::{RuntimeError, execute_proto, execute_proto_with_environment};
use elara_stdlib::StdLibProfile;

use crate::stdlib::runtime_environment_for_stdlib;

/// Error returned by the simple source evaluation path.
#[derive(Clone, Debug, PartialEq)]
pub enum EvalError {
    /// Compile-time diagnostics.
    Diagnostics(Vec<Diagnostic>),
    /// Runtime interpreter error.
    Runtime(RuntimeError),
}

/// Evaluates a simple Lua chunk through parser, compiler, verifier, and interpreter.
pub fn eval_simple_source(source: SourceId, input: &str) -> Result<Vec<Value>, EvalError> {
    let compiled = compile_simple_chunk(source, input);
    if !compiled.diagnostics.is_empty() {
        return Err(EvalError::Diagnostics(compiled.diagnostics));
    }

    let proto = compiled.proto.expect("compiler succeeded without a proto");
    execute_proto(&proto).map_err(EvalError::Runtime)
}

/// Evaluates a simple Lua chunk with implemented native standard libraries.
pub fn eval_simple_source_with_stdlib(
    source: SourceId,
    input: &str,
    profile: &StdLibProfile,
) -> Result<Vec<Value>, EvalError> {
    let compiled = compile_simple_chunk(source, input);
    if !compiled.diagnostics.is_empty() {
        return Err(EvalError::Diagnostics(compiled.diagnostics));
    }

    let proto = compiled.proto.expect("compiler succeeded without a proto");
    let environment = runtime_environment_for_stdlib(profile);
    execute_proto_with_environment(&proto, environment)
        .map(|output| output.values)
        .map_err(EvalError::Runtime)
}

#[cfg(test)]
mod tests {
    use elara_core::{SourceId, Value};
    use elara_interp::RuntimeErrorKind;

    use elara_stdlib::{StdLib, StdLibProfile};

    use crate::{EvalError, eval_simple_source, eval_simple_source_with_stdlib};

    #[test]
    fn eval_simple_returns_42_from_source() {
        assert_eq!(
            eval_simple_source(SourceId::new(0), "return 42"),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_returns_arithmetic_from_source() {
        assert_eq!(
            eval_simple_source(SourceId::new(0), "return 1 + 2 * 3"),
            Ok(vec![Value::integer(7)])
        );
    }

    #[test]
    fn eval_simple_returns_recursive_self_reference() {
        let values = eval_simple_source(
            SourceId::new(0),
            "local function self()\n  return self\nend\nreturn self()",
        )
        .expect("eval should pass");

        assert_eq!(values.len(), 1);
        assert!(values[0].is_closure());
    }

    #[test]
    fn eval_simple_executes_if_else() {
        assert_eq!(
            eval_simple_source(SourceId::new(0), "if false then return 1 else return 2 end"),
            Ok(vec![Value::integer(2)])
        );
    }

    #[test]
    fn eval_simple_executes_while_break() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local x = 0\nwhile true do\n  x = x + 1\n  break\nend\nreturn x",
            ),
            Ok(vec![Value::integer(1)])
        );
    }

    #[test]
    fn eval_simple_executes_repeat_until() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local x = 0\nrepeat\n  x = x + 1\nuntil true\nreturn x",
            ),
            Ok(vec![Value::integer(1)])
        );
    }

    #[test]
    fn eval_simple_executes_numeric_for_positive_step() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local sum = 0\nfor i = 1, 3 do\n  sum = sum + i\nend\nreturn sum",
            ),
            Ok(vec![Value::integer(6)])
        );
    }

    #[test]
    fn eval_simple_executes_numeric_for_negative_step() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local sum = 0\nfor i = 3, 1, -1 do\n  sum = sum + i\nend\nreturn sum",
            ),
            Ok(vec![Value::integer(6)])
        );
    }

    #[test]
    fn eval_simple_executes_generic_for_iterator_result() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local function once()\n  return 7\nend\nfor x in once do\n  return x\nend\nreturn 0",
            ),
            Ok(vec![Value::integer(7)])
        );
    }

    #[test]
    fn eval_simple_skips_generic_for_nil_iterator_result() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local function done()\n  return nil\nend\nfor x in done do\n  return 99\nend\nreturn 42",
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_returns_table_constructor() {
        let values = eval_simple_source(SourceId::new(0), "return { 1, named = 2, [3] = 4 }")
            .expect("eval should pass");

        assert_eq!(values.len(), 1);
        assert!(values[0].is_table());
    }

    #[test]
    fn eval_simple_executes_table_access_by_index() {
        assert_eq!(
            eval_simple_source(SourceId::new(0), "local t = {}\nt[1] = 42\nreturn t[1]"),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_executes_table_access_by_field() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local t = {}\nt.answer = 42\nreturn t.answer",
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_executes_declared_global_access() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "global answer = 41\nanswer = answer + 1\nreturn answer",
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_executes_implicit_global_access() {
        assert_eq!(
            eval_simple_source(SourceId::new(0), "answer = 42\nreturn answer"),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_exposes_default_env_table() {
        assert_eq!(
            eval_simple_source(SourceId::new(0), "answer = 42\nreturn _ENV.answer"),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_nested_function_captures_default_env() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "answer = 42\nlocal function read()\n  return answer\nend\nreturn read()",
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_executes_local_env_global_read() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local _ENV = { answer = 42 }\nreturn answer"
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_executes_local_env_global_write() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "local _ENV = {}\nanswer = 42\nreturn _ENV.answer",
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_rejects_declared_global_init_in_defined_local_env() {
        let error = eval_simple_source(
            SourceId::new(0),
            "local _ENV = { answer = 1 }\nglobal answer = 2\nreturn answer",
        )
        .unwrap_err();

        match error {
            EvalError::Runtime(error)
                if error.kind() == &RuntimeErrorKind::GlobalAlreadyDefined => {}
            EvalError::Runtime(error) => panic!("expected global error, got {error:?}"),
            EvalError::Diagnostics(diagnostics) => {
                panic!("expected runtime error, got diagnostics {diagnostics:?}")
            }
        }
    }

    #[test]
    fn eval_simple_executes_global_function_declaration() {
        assert_eq!(
            eval_simple_source(
                SourceId::new(0),
                "global function answer()\n  return 42\nend\nreturn answer()",
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_abs() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.abs(-7)", &profile),
            Ok(vec![Value::integer(7)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_max() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.max(1, 7, 3)", &profile),
            Ok(vec![Value::integer(7)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_type() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        let values =
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.type(7)", &profile)
                .expect("math.type should execute");

        assert_eq!(values.len(), 1);
        assert!(values[0].is_string());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.type(false)", &profile),
            Ok(vec![Value::nil()])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_random() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.random(1, 1)", &profile),
            Ok(vec![Value::integer(1)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_randomseed() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return math.randomseed(7, 9)",
                &profile,
            ),
            Ok(vec![Value::integer(7)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_trig() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return math.sin(0) + math.cos(0) + math.tan(0)",
                &profile,
            ),
            Ok(vec![Value::float(1.0)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_inverse_trig() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return math.asin(0) + math.acos(1) + math.atan(0)",
                &profile,
            ),
            Ok(vec![Value::float(0.0)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_angle_conversion() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return math.deg(math.rad(180))",
                &profile,
            ),
            Ok(vec![Value::float(180.0)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_exp_log() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return math.exp(0) + math.log(8, 2)",
                &profile,
            ),
            Ok(vec![Value::float(4.0)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_fmod() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.fmod(7, 3)", &profile),
            Ok(vec![Value::integer(1)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_ult() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return math.ult(1, 2)", &profile),
            Ok(vec![Value::boolean(true)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_math_tointeger() {
        let profile = StdLibProfile::Custom([StdLib::Math].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return math.tointeger(7.0), math.tointeger(7.5)",
                &profile,
            ),
            Ok(vec![Value::integer(7), Value::nil()])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_assert() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return assert(true, 42)", &profile),
            Ok(vec![Value::boolean(true)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_error() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());
        let error =
            eval_simple_source_with_stdlib(SourceId::new(0), "return error('boom')", &profile)
                .expect_err("error should raise");

        match error {
            EvalError::Runtime(error) => assert_eq!(error.message(), "boom"),
            EvalError::Diagnostics(diagnostics) => {
                panic!("expected runtime error, got diagnostics {diagnostics:?}")
            }
        }
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_rawequal() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return rawequal(7, 7.0)", &profile),
            Ok(vec![Value::boolean(true)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_type() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        let values = eval_simple_source_with_stdlib(SourceId::new(0), "return type(7)", &profile)
            .expect("type should execute");

        assert_eq!(values.len(), 1);
        assert!(values[0].is_string());
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_print() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local _ = print('hello', 7)\nreturn 1",
                &profile,
            ),
            Ok(vec![Value::integer(1)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_raw_functions() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = {}\nlocal _ = rawset(t, 'name', 42)\nreturn rawget(t, 'name')",
                &profile,
            ),
            Ok(vec![Value::integer(42)])
        );
        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return rawlen({1, 2, 3})", &profile),
            Ok(vec![Value::integer(3)])
        );
        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return rawlen('abc')", &profile),
            Ok(vec![Value::integer(3)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_metatable_functions() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = {}\nlocal mt = { __metatable = 'locked' }\nlocal _ = setmetatable(t, mt)\nreturn rawequal(getmetatable(t), 'locked')",
                &profile,
            ),
            Ok(vec![Value::boolean(true)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_next() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = {10, 20}\nreturn next(t)",
                &profile,
            ),
            Ok(vec![Value::integer(1)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_tonumber() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return tonumber('2a', 16)", &profile),
            Ok(vec![Value::integer(42)])
        );
        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return tonumber('bad')", &profile),
            Ok(vec![Value::nil()])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_base_tostring() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return tonumber(tostring(42))",
                &profile,
            ),
            Ok(vec![Value::integer(42)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_len() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return string.len('abcd')", &profile),
            Ok(vec![Value::integer(4)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_byte() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return string.byte('AZ', 2)",
                &profile,
            ),
            Ok(vec![Value::integer(90)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_char() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return string.byte(string.char(65, 90), 2)",
                &profile,
            ),
            Ok(vec![Value::integer(90)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_reverse() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        let values = eval_simple_source_with_stdlib(
            SourceId::new(0),
            "return string.reverse('abc')",
            &profile,
        )
        .expect("string.reverse should execute");

        assert_eq!(values.len(), 1);
        assert!(values[0].is_string());
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_find() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local i = string.find('abcabc', 'ca')\nreturn i",
                &profile,
            ),
            Ok(vec![Value::integer(3)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_format() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return string.len(string.format('%5.2s:%+05d:%#06x', 'abcd', 7, 255))",
                &profile,
            ),
            Ok(vec![Value::integer(18)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_gsub() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return string.len(string.gsub('abab', 'a', 'x'))",
                &profile,
            ),
            Ok(vec![Value::integer(4)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_match() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return string.len(string.match('abcabc', 'ca'))",
                &profile,
            ),
            Ok(vec![Value::integer(2)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_rep() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return string.len(string.rep('ab', 3, ','))",
                &profile,
            ),
            Ok(vec![Value::integer(8)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_string_sub() {
        let profile = StdLibProfile::Custom([StdLib::String].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "return string.len(string.sub('abcdef', 2, -2))",
                &profile,
            ),
            Ok(vec![Value::integer(4)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_table_pack() {
        let profile = StdLibProfile::Custom([StdLib::Table].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = table.pack(1, nil, 3)\nreturn t.n + t[3]",
                &profile,
            ),
            Ok(vec![Value::integer(6)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_table_unpack() {
        let profile = StdLibProfile::Custom([StdLib::Table].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = table.pack(1, 2, 3)\nreturn table.unpack(t, 2, 2)",
                &profile,
            ),
            Ok(vec![Value::integer(2)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_table_insert() {
        let profile = StdLibProfile::Custom([StdLib::Table].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = table.pack(1, 3)\nlocal _ = table.insert(t, 2, 2)\nreturn t[2] + t[3]",
                &profile,
            ),
            Ok(vec![Value::integer(5)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_table_remove() {
        let profile = StdLibProfile::Custom([StdLib::Table].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = table.pack(1, 2, 3)\nlocal v = table.remove(t, 2)\nreturn v + t[2]",
                &profile,
            ),
            Ok(vec![Value::integer(5)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_table_move() {
        let profile = StdLibProfile::Custom([StdLib::Table].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local source = table.pack(1, 2, 3)\nlocal dest = table.pack()\nlocal moved = table.move(source, 2, 3, 1, dest)\nreturn moved[1] + moved[2]",
                &profile,
            ),
            Ok(vec![Value::integer(5)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_table_concat() {
        let profile = StdLibProfile::Custom([StdLib::String, StdLib::Table].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = table.pack('a', 'b', 'c')\nreturn string.len(table.concat(t, '-'))",
                &profile,
            ),
            Ok(vec![Value::integer(5)])
        );
    }

    #[test]
    fn eval_simple_with_stdlib_executes_table_sort() {
        let profile = StdLibProfile::Custom([StdLib::Table].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(
                SourceId::new(0),
                "local t = table.pack(3, 1, 2)\nlocal _ = table.sort(t)\nreturn t[1] + t[2] + t[3]",
                &profile,
            ),
            Ok(vec![Value::integer(6)])
        );
    }

    #[test]
    fn eval_simple_rejects_global_function_when_defined() {
        let error = eval_simple_source(
            SourceId::new(0),
            "answer = 1\nglobal function answer()\n  return 42\nend",
        )
        .unwrap_err();

        match error {
            EvalError::Runtime(error)
                if error.kind() == &RuntimeErrorKind::GlobalAlreadyDefined => {}
            EvalError::Runtime(error) => panic!("expected global error, got {error:?}"),
            EvalError::Diagnostics(diagnostics) => {
                panic!("expected runtime error, got diagnostics {diagnostics:?}")
            }
        }
    }

    #[test]
    fn eval_simple_reports_compile_diagnostics() {
        let error =
            eval_simple_source(SourceId::new(0), "global answer\nreturn missing").unwrap_err();

        match error {
            EvalError::Diagnostics(diagnostics) => {
                assert_eq!(diagnostics[0].message(), "variable 'missing' not declared");
            }
            EvalError::Runtime(error) => panic!("expected diagnostics, got {error:?}"),
        }
    }
}
