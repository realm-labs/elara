//! Simple source evaluation path.

use elara_compiler::compile_simple_chunk;
use elara_core::{Diagnostic, SourceId, Value};
use elara_interp::{RuntimeError, execute_proto};

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

#[cfg(test)]
mod tests {
    use elara_core::{SourceId, Value};

    use crate::{EvalError, eval_simple_source};

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
    fn eval_simple_reports_compile_diagnostics() {
        let error = eval_simple_source(SourceId::new(0), "x = 1").unwrap_err();

        match error {
            EvalError::Diagnostics(diagnostics) => {
                assert_eq!(
                    diagnostics[0].message(),
                    "assignment target is not a declared local"
                );
            }
            EvalError::Runtime(error) => panic!("expected diagnostics, got {error:?}"),
        }
    }
}
