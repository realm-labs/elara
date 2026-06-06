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
    fn eval_simple_reports_compile_diagnostics() {
        let error = eval_simple_source(SourceId::new(0), "x = 1").unwrap_err();

        match error {
            EvalError::Diagnostics(diagnostics) => {
                assert_eq!(
                    diagnostics[0].message(),
                    "unsupported statement in simple expression compiler"
                );
            }
            EvalError::Runtime(error) => panic!("expected diagnostics, got {error:?}"),
        }
    }
}
