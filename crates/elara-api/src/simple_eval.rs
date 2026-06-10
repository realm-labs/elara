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
    fn eval_simple_with_stdlib_executes_base_assert() {
        let profile = StdLibProfile::Custom([StdLib::Base].into_iter().collect());

        assert_eq!(
            eval_simple_source_with_stdlib(SourceId::new(0), "return assert(true, 42)", &profile),
            Ok(vec![Value::boolean(true)])
        );
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
