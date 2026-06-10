use elara_core::{LuaInteger, Value};

use super::{
    LuaRandomState, math_cos, math_deg, math_exp, math_fmod, math_log, math_modf, math_rad,
    math_sin, math_tan,
};
use crate::{NativeError, NativeRuntime};

#[derive(Default)]
struct TestRuntime {
    random: LuaRandomState,
}

impl NativeRuntime for TestRuntime {
    fn intern_short_string(&mut self, _bytes: &[u8]) -> Result<Value, NativeError> {
        unreachable!("math tests do not intern strings")
    }

    fn short_string_bytes(&self, _value: Value) -> Option<&[u8]> {
        None
    }

    fn next_random_u64(&mut self) -> Result<u64, NativeError> {
        Ok(self.random.next_u64())
    }
}

fn call(function: crate::NativeStdFunction, args: &[Value]) -> Vec<Value> {
    function(&mut TestRuntime::default(), args).expect("native should pass")
}

#[test]
fn math_trig_functions_return_floats() {
    assert_eq!(
        call(math_sin, &[Value::integer(0)]),
        vec![Value::float(0.0)]
    );
    assert_eq!(
        call(math_cos, &[Value::integer(0)]),
        vec![Value::float(1.0)]
    );
    assert_eq!(
        call(math_tan, &[Value::integer(0)]),
        vec![Value::float(0.0)]
    );
}

#[test]
fn math_angle_conversion_functions_return_floats() {
    assert_eq!(
        call(math_deg, &[Value::float(std::f64::consts::PI)]),
        vec![Value::float(180.0)]
    );
    assert_eq!(
        call(math_rad, &[Value::integer(180)]),
        vec![Value::float(std::f64::consts::PI)]
    );
}

#[test]
fn math_exp_and_log_return_floats() {
    assert_eq!(
        call(math_exp, &[Value::integer(0)]),
        vec![Value::float(1.0)]
    );
    assert_eq!(
        call(math_log, &[Value::integer(1)]),
        vec![Value::float(0.0)]
    );
    assert_eq!(
        call(math_log, &[Value::integer(8), Value::integer(2)]),
        vec![Value::float(3.0)]
    );
    assert_eq!(
        call(math_log, &[Value::integer(100), Value::integer(10)]),
        vec![Value::float(2.0)]
    );
}

#[test]
fn math_fmod_matches_integer_and_float_paths() {
    assert_eq!(
        call(math_fmod, &[Value::integer(7), Value::integer(3)]),
        vec![Value::integer(1)]
    );
    assert_eq!(
        call(
            math_fmod,
            &[Value::integer(LuaInteger::MIN), Value::integer(-1)]
        ),
        vec![Value::integer(0)]
    );
    assert_eq!(
        call(math_fmod, &[Value::float(7.5), Value::integer(2)]),
        vec![Value::float(1.5)]
    );
}

#[test]
fn math_modf_returns_integral_and_fractional_parts() {
    assert_eq!(
        call(math_modf, &[Value::integer(7)]),
        vec![Value::integer(7), Value::float(0.0)]
    );
    assert_eq!(
        call(math_modf, &[Value::float(-3.25)]),
        vec![Value::integer(-3), Value::float(-0.25)]
    );
}
