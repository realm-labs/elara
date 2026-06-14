use elara_core::Value;

use crate::{NativeErrorKind, NativeRuntime, StdLib, native_functions};

#[derive(Default)]
struct TestRuntime {
    strings: Vec<Box<[u8]>>,
    debug_upvalue: Option<(Value, Value)>,
    debug_upvalue_request: Option<(Value, i64)>,
    debug_setupvalue: Option<Value>,
    debug_setupvalue_request: Option<(Value, i64, Value)>,
    debug_upvalueid: Option<Value>,
    debug_upvalueid_request: Option<(Value, i64)>,
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
        unreachable!("debug upvalue tests do not intern short strings")
    }

    fn intern_string(&mut self, bytes: &[u8]) -> Result<Value, crate::NativeError> {
        Ok(self.push_string(bytes))
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        let index = value.as_closure_index()? as usize;
        self.strings.get(index).map(Box::as_ref)
    }

    fn debug_getupvalue(
        &mut self,
        function: Value,
        index: i64,
    ) -> Result<Option<(Value, Value)>, crate::NativeError> {
        self.debug_upvalue_request = Some((function, index));
        Ok(self.debug_upvalue)
    }

    fn debug_setupvalue(
        &mut self,
        function: Value,
        index: i64,
        value: Value,
    ) -> Result<Option<Value>, crate::NativeError> {
        self.debug_setupvalue_request = Some((function, index, value));
        Ok(self.debug_setupvalue)
    }

    fn debug_upvalueid(
        &mut self,
        function: Value,
        index: i64,
    ) -> Result<Option<Value>, crate::NativeError> {
        self.debug_upvalueid_request = Some((function, index));
        Ok(self.debug_upvalueid)
    }
}

#[test]
fn debug_getupvalue_forwards_function_and_index_queries() {
    let function = function("getupvalue");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);
    let name = runtime.push_string(b"x");
    runtime.debug_upvalue = Some((name, Value::integer(42)));

    assert_eq!(
        function(&mut runtime, &[target, Value::integer(1)]).expect("debug.getupvalue should pass"),
        vec![name, Value::integer(42)]
    );
    assert_eq!(runtime.debug_upvalue_request, Some((target, 1)));
}

#[test]
fn debug_getupvalue_returns_nil_for_absent_upvalues() {
    let function = function("getupvalue");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);

    assert_eq!(
        function(&mut runtime, &[target, Value::integer(2)]).expect("debug.getupvalue should pass"),
        vec![Value::nil()]
    );
    assert_eq!(runtime.debug_upvalue_request, Some((target, 2)));
}

#[test]
fn debug_getupvalue_validates_arguments() {
    let function = function("getupvalue");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);

    assert_eq!(
        function(&mut runtime, &[])
            .expect_err("missing function")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1), Value::integer(1)])
            .expect_err("bad function")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "function",
        }
    );
    assert_eq!(
        function(&mut runtime, &[target])
            .expect_err("missing index")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
    assert_eq!(
        function(&mut runtime, &[target, Value::boolean(false)])
            .expect_err("bad index")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        }
    );
}

#[test]
fn debug_setupvalue_forwards_function_index_and_value() {
    let function = function("setupvalue");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);
    let name = runtime.push_string(b"x");
    runtime.debug_setupvalue = Some(name);

    assert_eq!(
        function(
            &mut runtime,
            &[target, Value::integer(1), Value::integer(42)]
        )
        .expect("debug.setupvalue should pass"),
        vec![name]
    );
    assert_eq!(
        runtime.debug_setupvalue_request,
        Some((target, 1, Value::integer(42)))
    );
}

#[test]
fn debug_setupvalue_returns_nil_for_absent_upvalues() {
    let function = function("setupvalue");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);

    assert_eq!(
        function(
            &mut runtime,
            &[target, Value::integer(2), Value::integer(99)]
        )
        .expect("debug.setupvalue should pass"),
        vec![Value::nil()]
    );
    assert_eq!(
        runtime.debug_setupvalue_request,
        Some((target, 2, Value::integer(99)))
    );
}

#[test]
fn debug_setupvalue_validates_arguments() {
    let function = function("setupvalue");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);

    assert_eq!(
        function(&mut runtime, &[])
            .expect_err("missing function")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        function(
            &mut runtime,
            &[Value::integer(1), Value::integer(1), Value::nil()]
        )
        .expect_err("bad function")
        .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "function",
        }
    );
    assert_eq!(
        function(&mut runtime, &[target])
            .expect_err("missing index")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
    assert_eq!(
        function(&mut runtime, &[target, Value::boolean(false), Value::nil()])
            .expect_err("bad index")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        }
    );
    assert_eq!(
        function(&mut runtime, &[target, Value::integer(1)])
            .expect_err("missing value")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 3 }
    );
}

#[test]
fn debug_upvalueid_forwards_function_and_index_queries() {
    let function = function("upvalueid");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);
    let id = Value::light_user_data(0x1234);
    runtime.debug_upvalueid = Some(id);

    assert_eq!(
        function(&mut runtime, &[target, Value::integer(1)]).expect("debug.upvalueid should pass"),
        vec![id]
    );
    assert_eq!(runtime.debug_upvalueid_request, Some((target, 1)));
}

#[test]
fn debug_upvalueid_returns_nil_for_absent_upvalues() {
    let function = function("upvalueid");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);

    assert_eq!(
        function(&mut runtime, &[target, Value::integer(2)]).expect("debug.upvalueid should pass"),
        vec![Value::nil()]
    );
    assert_eq!(runtime.debug_upvalueid_request, Some((target, 2)));
}

#[test]
fn debug_upvalueid_validates_arguments() {
    let function = function("upvalueid");
    let mut runtime = TestRuntime::default();
    let target = Value::native_function_index(3);

    assert_eq!(
        function(&mut runtime, &[])
            .expect_err("missing function")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 1 }
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1), Value::integer(1)])
            .expect_err("bad function")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "function",
        }
    );
    assert_eq!(
        function(&mut runtime, &[target])
            .expect_err("missing index")
            .kind(),
        &NativeErrorKind::MissingArgument { index: 2 }
    );
    assert_eq!(
        function(&mut runtime, &[target, Value::boolean(false)])
            .expect_err("bad index")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "integer",
        }
    );
}

fn function(name: &str) -> crate::NativeStdFunction {
    native_functions(StdLib::Debug)
        .iter()
        .find(|function| function.descriptor().name() == name)
        .expect("debug native function should exist")
        .function()
}
