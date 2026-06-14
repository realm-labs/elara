use elara_core::Value;

use crate::{DebugHookState, NativeErrorKind, NativeRuntime, StdLib, native_functions};

#[derive(Default)]
struct TestRuntime {
    strings: Vec<Box<[u8]>>,
    debug_hook: Option<DebugHookState>,
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
        unreachable!("debug hook natives use intern_string instead")
    }

    fn intern_string(&mut self, bytes: &[u8]) -> Result<Value, crate::NativeError> {
        Ok(self.push_string(bytes))
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        let index = value.as_closure_index()? as usize;
        self.strings.get(index).map(Box::as_ref)
    }

    fn debug_gethook(&mut self) -> Result<Option<DebugHookState>, crate::NativeError> {
        Ok(self.debug_hook.clone())
    }

    fn debug_sethook(&mut self, hook: DebugHookState) -> Result<(), crate::NativeError> {
        self.debug_hook = Some(hook);
        Ok(())
    }

    fn debug_clearhook(&mut self) -> Result<(), crate::NativeError> {
        self.debug_hook = None;
        Ok(())
    }
}

#[test]
fn debug_gethook_returns_nil_without_installed_hook() {
    let function = function("gethook");
    let mut runtime = TestRuntime::default();

    assert_eq!(
        function(&mut runtime, &[]).expect("debug.gethook should pass"),
        vec![Value::nil()]
    );
    assert_eq!(
        function(&mut runtime, &[Value::integer(1)]).expect("non-thread arg should pass"),
        vec![Value::nil()]
    );
}

#[test]
fn debug_gethook_returns_installed_hook() {
    let function = function("gethook");
    let mut runtime = TestRuntime::default();
    let hook = Value::native_function_index(1);
    runtime.debug_hook = Some(DebugHookState {
        function: hook,
        mask: b"cr".to_vec(),
        count: 7,
    });

    let values = function(&mut runtime, &[]).expect("debug.gethook should pass");

    assert_eq!(values.len(), 3);
    assert_eq!(values[0], hook);
    assert_eq!(runtime.short_string_bytes(values[1]), Some(&b"cr"[..]));
    assert_eq!(values[2], Value::integer(7));
}

#[test]
fn debug_sethook_clears_hooks_without_results() {
    let function = function("sethook");
    let mut runtime = TestRuntime {
        debug_hook: Some(DebugHookState {
            function: Value::native_function_index(1),
            mask: b"l".to_vec(),
            count: 1,
        }),
        ..TestRuntime::default()
    };

    assert_eq!(
        function(&mut runtime, &[]).expect("debug.sethook should clear hooks"),
        Vec::<Value>::new()
    );
    assert_eq!(runtime.debug_hook, None);
    runtime.debug_hook = Some(DebugHookState {
        function: Value::native_function_index(2),
        mask: b"c".to_vec(),
        count: 0,
    });
    assert_eq!(
        function(&mut runtime, &[Value::nil()]).expect("debug.sethook(nil) should clear hooks"),
        Vec::<Value>::new()
    );
    assert_eq!(runtime.debug_hook, None);
}

#[test]
fn debug_sethook_installs_normalized_hook_state() {
    let function = function("sethook");
    let mut runtime = TestRuntime::default();
    let hook = Value::native_function_index(1);
    let mask = runtime.push_string(b"lrcx");

    assert_eq!(
        function(&mut runtime, &[hook, mask, Value::integer(5)])
            .expect("debug.sethook should install hooks"),
        Vec::<Value>::new()
    );
    assert_eq!(
        runtime.debug_hook,
        Some(DebugHookState {
            function: hook,
            mask: b"crl".to_vec(),
            count: 5,
        })
    );
}

#[test]
fn debug_sethook_validates_arguments() {
    let function = function("sethook");
    let mut runtime = TestRuntime::default();
    let hook = Value::native_function_index(1);
    let mask = runtime.push_string(b"c");

    assert_eq!(
        function(&mut runtime, &[Value::integer(1)])
            .expect_err("non-function hook should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 1,
            expected: "function",
        }
    );
    assert_eq!(
        function(&mut runtime, &[hook, Value::integer(1)])
            .expect_err("non-string mask should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 2,
            expected: "string",
        }
    );
    assert_eq!(
        function(&mut runtime, &[hook, mask, Value::boolean(false)])
            .expect_err("non-integer count should fail")
            .kind(),
        &NativeErrorKind::TypeError {
            index: 3,
            expected: "integer",
        }
    );
}

fn function(name: &str) -> crate::NativeStdFunction {
    native_functions(StdLib::Debug)
        .iter()
        .find(|function| function.descriptor().name() == name)
        .expect("debug native should exist")
        .function()
}
