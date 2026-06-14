use elara_core::Value;

use crate::{NativeErrorKind, NativeRuntime, StdLib, native_functions};

#[derive(Default)]
struct TestRuntime {
    strings: Vec<Box<[u8]>>,
    debug_local: Option<(Value, Value)>,
    debug_local_request: Option<(i64, i64)>,
    debug_setlocal: Option<Value>,
    debug_setlocal_request: Option<(i64, i64, Value)>,
}

impl TestRuntime {
    fn push_string(&mut self, bytes: &[u8]) -> Value {
        let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
        self.strings.push(bytes.into());
        Value::closure_index(index)
    }
}

impl NativeRuntime for TestRuntime {
    fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, crate::NativeError> {
        unreachable!("debug local tests do not intern short strings")
    }

    fn intern_string(&mut self, bytes: &[u8]) -> Result<Value, crate::NativeError> {
        Ok(self.push_string(bytes))
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        let index = value.as_closure_index()? as usize;
        self.strings.get(index).map(Box::as_ref)
    }

    fn debug_getlocal(
        &mut self,
        level: i64,
        local: i64,
    ) -> Result<Option<(Value, Value)>, crate::NativeError> {
        self.debug_local_request = Some((level, local));
        Ok(self.debug_local)
    }

    fn debug_setlocal(
        &mut self,
        level: i64,
        local: i64,
        value: Value,
    ) -> Result<Option<Value>, crate::NativeError> {
        self.debug_setlocal_request = Some((level, local, value));
        Ok(self.debug_setlocal)
    }
}

#[test]
fn debug_getlocal_forwards_level_and_index_queries() {
    let function = function("getlocal");
    let mut runtime = TestRuntime::default();
    let name = runtime.push_string(b"x");
    runtime.debug_local = Some((name, Value::integer(42)));

    assert_eq!(
        function(&mut runtime, &[Value::integer(1), Value::integer(2)])
            .expect("debug.getlocal should pass"),
        vec![name, Value::integer(42)]
    );
    assert_eq!(runtime.debug_local_request, Some((1, 2)));
}

#[test]
fn debug_getlocal_returns_nil_for_absent_locals_and_function_targets() {
    let function = function("getlocal");
    let mut runtime = TestRuntime::default();
    let target = Value::closure_index(3);

    assert_eq!(
        function(&mut runtime, &[Value::integer(1), Value::integer(9)])
            .expect("debug.getlocal should pass"),
        vec![Value::nil()]
    );
    assert_eq!(runtime.debug_local_request, Some((1, 9)));

    assert_eq!(
        function(&mut runtime, &[target, Value::integer(1)])
            .expect("function-target locals are not materialized yet"),
        vec![Value::nil()]
    );
}

#[test]
fn debug_getlocal_validates_arguments() {
    let function = function("getlocal");
    let mut runtime = TestRuntime::default();

    assert_eq!(
        function(&mut runtime, &[])
            .expect_err("missing level")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        function(&mut runtime, &[Value::boolean(false), Value::integer(1)])
            .expect_err("bad level")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "function or integer",
        }
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1)])
            .expect_err("missing local index")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1), Value::boolean(false)])
            .expect_err("bad local index")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        }
    );
}

#[test]
fn debug_setlocal_forwards_level_index_and_value() {
    let function = function("setlocal");
    let mut runtime = TestRuntime::default();
    let name = runtime.push_string(b"x");
    runtime.debug_setlocal = Some(name);

    assert_eq!(
        function(
            &mut runtime,
            &[Value::integer(1), Value::integer(2), Value::integer(99)]
        )
        .expect("debug.setlocal should pass"),
        vec![name]
    );
    assert_eq!(
        runtime.debug_setlocal_request,
        Some((1, 2, Value::integer(99)))
    );
}

#[test]
fn debug_setlocal_returns_nil_for_absent_locals() {
    let function = function("setlocal");
    let mut runtime = TestRuntime::default();

    assert_eq!(
        function(
            &mut runtime,
            &[Value::integer(1), Value::integer(9), Value::integer(99)]
        )
        .expect("debug.setlocal should pass"),
        vec![Value::nil()]
    );
    assert_eq!(
        runtime.debug_setlocal_request,
        Some((1, 9, Value::integer(99)))
    );
}

#[test]
fn debug_setlocal_validates_arguments() {
    let function = function("setlocal");
    let mut runtime = TestRuntime::default();

    assert_eq!(
        function(&mut runtime, &[])
            .expect_err("missing level")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        function(&mut runtime, &[Value::boolean(false), Value::integer(1)])
            .expect_err("bad level")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "integer",
        }
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1)])
            .expect_err("missing local index")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1), Value::boolean(false)])
            .expect_err("bad local index")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        }
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1), Value::integer(1)])
            .expect_err("missing value")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 3 }
    );
}

fn function(name: &str) -> crate::NativeStdFunction {
    native_functions(StdLib::Debug)
        .iter()
        .find(|function| function.descriptor().name() == name)
        .map(|function| function.function())
        .expect("debug native should be registered")
}
