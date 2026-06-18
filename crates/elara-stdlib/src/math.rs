//! Executable math-library natives.

use elara_core::{LuaFloat, Value, float_to_integer_exact};

use crate::{
    FunctionSpec, NativeError, NativeErrorKind, NativeFunctionSpec, NativeRuntime, StdLib,
    number::parse_standard_number,
};

const PI: LuaFloat = std::f64::consts::PI;

/// Executable math-library functions currently implemented.
pub const MATH_NATIVE_FUNCTIONS: &[NativeFunctionSpec] = &[
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "abs"), math_abs),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "acos"), math_acos),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "asin"), math_asin),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "atan"), math_atan),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "ceil"), math_ceil),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "cos"), math_cos),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "deg"), math_deg),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "exp"), math_exp),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "floor"), math_floor),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "fmod"), math_fmod),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "frexp"), math_frexp),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "ldexp"), math_ldexp),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "log"), math_log),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "max"), math_max),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "min"), math_min),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "modf"), math_modf),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "random"), math_random),
    NativeFunctionSpec::new(
        FunctionSpec::new(StdLib::Math, "randomseed"),
        math_randomseed,
    ),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "rad"), math_rad),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "sin"), math_sin),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "sqrt"), math_sqrt),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "tan"), math_tan),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "tointeger"), math_tointeger),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "type"), math_type),
    NativeFunctionSpec::new(FunctionSpec::new(StdLib::Math, "ult"), math_ult),
];

/// Math-library constant fields.
pub const MATH_CONSTANTS: &[(&str, Value)] = &[
    ("pi", Value::float(PI)),
    ("huge", Value::float(LuaFloat::INFINITY)),
    ("maxinteger", Value::integer(i64::MAX)),
    ("mininteger", Value::integer(i64::MIN)),
];

/// Lua 5.5-style xoshiro256** random state.
#[derive(Clone, Debug)]
pub struct LuaRandomState {
    state: [u64; 4],
}

impl LuaRandomState {
    /// Creates a random state from two Lua integer seeds.
    #[must_use]
    pub fn from_seeds(seed1: u64, seed2: u64) -> Self {
        let mut state = Self {
            state: [seed1, 0xff, seed2, 0],
        };
        for _ in 0..16 {
            state.next_u64();
        }
        state
    }

    /// Returns the next 64-bit random value.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;
        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);
        result
    }
}

impl Default for LuaRandomState {
    fn default() -> Self {
        Self::from_seeds(0x0123_4567_89ab_cdef, 0xfedc_ba98_7654_3210)
    }
}

fn math_abs(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(runtime, args, 1)?;
    let result = if let Some(integer) = value.as_integer() {
        if integer < 0 {
            Value::integer(integer.wrapping_neg())
        } else {
            value
        }
    } else {
        Value::float(
            value
                .as_float()
                .expect("number_arg accepted only numbers")
                .abs(),
        )
    };
    Ok(vec![result])
}

fn math_floor(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(runtime, args, 1)?;
    let result = if value.as_integer().is_some() {
        value
    } else {
        number_result(
            value
                .to_float()
                .expect("number_arg accepted only numbers")
                .floor(),
        )
    };
    Ok(vec![result])
}

fn math_ceil(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(runtime, args, 1)?;
    let result = if value.as_integer().is_some() {
        value
    } else {
        number_result(
            value
                .to_float()
                .expect("number_arg accepted only numbers")
                .ceil(),
        )
    };
    Ok(vec![result])
}

fn math_sqrt(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(runtime, args, 1)?;
    Ok(vec![Value::float(
        value
            .to_float()
            .expect("number_arg accepted only numbers")
            .sqrt(),
    )])
}

fn math_asin(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)?.asin(),
    )])
}

fn math_acos(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)?.acos(),
    )])
}

fn math_atan(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let y = number_float_arg(runtime, args, 1)?;
    let x = args.get(1).map_or(Ok(1.0), |value| {
        if value.is_nil() {
            Ok(1.0)
        } else {
            number_value_to_float(runtime, *value, 2)
        }
    })?;
    Ok(vec![Value::float(y.atan2(x))])
}

fn math_sin(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)?.sin(),
    )])
}

fn math_cos(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)?.cos(),
    )])
}

fn math_tan(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)?.tan(),
    )])
}

fn math_deg(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)? * (180.0 / PI),
    )])
}

fn math_rad(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)? * (PI / 180.0),
    )])
}

fn math_exp(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::float(
        number_float_arg(runtime, args, 1)?.exp(),
    )])
}

fn math_log(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_float_arg(runtime, args, 1)?;
    let result = match args.get(1) {
        None => value.ln(),
        Some(base) if base.is_nil() => value.ln(),
        Some(base) => {
            let base = number_value_to_float(runtime, *base, 2)?;
            if base == 2.0 {
                value.log2()
            } else if base == 10.0 {
                value.log10()
            } else {
                value.ln() / base.ln()
            }
        }
    };
    Ok(vec![Value::float(result)])
}

fn math_fmod(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let left = number_arg(runtime, args, 1)?;
    let right = number_arg(runtime, args, 2)?;
    if let (Some(left), Some(right)) = (left.as_integer(), right.as_integer()) {
        if right == 0 {
            return Err(NativeErrorKind::ArgumentOutOfRange { index: 2 }.into());
        }
        if right == -1 {
            return Ok(vec![Value::integer(0)]);
        }
        return Ok(vec![Value::integer(left % right)]);
    }
    Ok(vec![Value::float(
        number_value_to_float(runtime, left, 1)? % number_value_to_float(runtime, right, 2)?,
    )])
}

fn math_frexp(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_float_arg(runtime, args, 1)?;
    if value == 0.0 || !value.is_finite() {
        return Ok(vec![Value::float(value), Value::integer(0)]);
    }
    let exponent = value.abs().log2().floor() as i64 + 1;
    let mantissa = value / 2.0_f64.powi(exponent as i32);
    Ok(vec![Value::float(mantissa), Value::integer(exponent)])
}

fn math_ldexp(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_float_arg(runtime, args, 1)?;
    let exponent = integer_arg(runtime, args, 2)?;
    let exponent =
        i32::try_from(exponent).map_err(|_| NativeErrorKind::ArgumentOutOfRange { index: 2 })?;
    Ok(vec![Value::float(value * 2.0_f64.powi(exponent))])
}

fn math_modf(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = number_arg(runtime, args, 1)?;
    if value.as_integer().is_some() {
        return Ok(vec![value, Value::float(0.0)]);
    }
    let value = number_value_to_float(runtime, value, 1)?;
    let integer = if value < 0.0 {
        value.ceil()
    } else {
        value.floor()
    };
    let fraction = if value == integer {
        0.0
    } else {
        value - integer
    };
    Ok(vec![number_result(integer), Value::float(fraction)])
}

fn math_ult(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    Ok(vec![Value::boolean(
        (integer_arg(runtime, args, 1)? as u64) < (integer_arg(runtime, args, 2)? as u64),
    )])
}

fn math_min(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    extrema_arg(runtime, args, Extrema::Min).map(|value| vec![value])
}

fn math_max(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    extrema_arg(runtime, args, Extrema::Max).map(|value| vec![value])
}

fn math_random(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let random = runtime.next_random_u64()?;
    let (low, high) = match args.len() {
        0 => return Ok(vec![Value::float(random_float(random))]),
        1 => {
            let high = integer_arg(runtime, args, 1)?;
            if high == 0 {
                return Ok(vec![Value::integer(random as i64)]);
            }
            (1, high)
        }
        2 => (
            integer_arg(runtime, args, 1)?,
            integer_arg(runtime, args, 2)?,
        ),
        _ => {
            return Err(NativeErrorKind::RuntimeError {
                message: "wrong number of arguments".into(),
            }
            .into());
        }
    };

    if low > high {
        return Err(NativeErrorKind::ArgumentOutOfRange { index: 1 }.into());
    }

    let span = (high as u64).wrapping_sub(low as u64);
    let projected = project_random(runtime, random, span)?;
    Ok(vec![Value::integer(
        projected.wrapping_add(low as u64) as i64
    )])
}

fn math_randomseed(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let (seed1, seed2) = match args.len() {
        0 => (runtime.random_seed()?, runtime.next_random_u64()?),
        1 => (integer_arg(runtime, args, 1)? as u64, 0),
        _ => (
            integer_arg(runtime, args, 1)? as u64,
            optional_integer_arg(runtime, args, 2, 0)? as u64,
        ),
    };
    runtime.set_random_seed(seed1, seed2)?;
    Ok(vec![
        Value::integer(seed1 as i64),
        Value::integer(seed2 as i64),
    ])
}

fn math_type(runtime: &mut dyn NativeRuntime, args: &[Value]) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    let result = if value.as_integer().is_some() {
        runtime.intern_short_string(b"integer")?
    } else if value.as_float().is_some() {
        runtime.intern_short_string(b"float")?
    } else {
        Value::nil()
    };
    Ok(vec![result])
}

fn math_tointeger(
    runtime: &mut dyn NativeRuntime,
    args: &[Value],
) -> Result<Vec<Value>, NativeError> {
    let value = *args
        .first()
        .ok_or(NativeErrorKind::MissingArgument { index: 1 })?;
    if let Some(integer) = to_integer(runtime, value) {
        Ok(vec![Value::integer(integer)])
    } else {
        Ok(vec![Value::nil()])
    }
}

fn integer_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<i64, NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    integer_value(runtime, value).ok_or(
        NativeErrorKind::TypeError {
            index,
            expected: "integer",
        }
        .into(),
    )
}

fn optional_integer_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
    default: i64,
) -> Result<i64, NativeError> {
    match args.get(index - 1) {
        Some(value) if value.is_nil() => Ok(default),
        Some(value) => integer_value(runtime, *value).ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "integer",
            }
            .into(),
        ),
        None => Ok(default),
    }
}

fn number_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Value, NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    number_value(runtime, value).ok_or(
        NativeErrorKind::TypeError {
            index,
            expected: "number",
        }
        .into(),
    )
}

fn number_float_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<LuaFloat, NativeError> {
    number_arg(runtime, args, index).map(|value| {
        value
            .to_float()
            .expect("number_arg accepted only number values")
    })
}

fn number_value_to_float(
    runtime: &dyn NativeRuntime,
    value: Value,
    index: usize,
) -> Result<LuaFloat, NativeError> {
    number_value(runtime, value)
        .and_then(Value::to_float)
        .ok_or(
            NativeErrorKind::TypeError {
                index,
                expected: "number",
            }
            .into(),
        )
}

fn number_result(value: LuaFloat) -> Value {
    float_to_integer_exact(value).map_or_else(|| Value::float(value), Value::integer)
}

fn to_integer(runtime: &dyn NativeRuntime, value: Value) -> Option<i64> {
    integer_value(runtime, value)
}

fn integer_value(runtime: &dyn NativeRuntime, value: Value) -> Option<i64> {
    value.to_integer_exact().or_else(|| {
        runtime
            .string_bytes(value)
            .and_then(parse_standard_number)
            .and_then(Value::to_integer_exact)
    })
}

fn number_value(runtime: &dyn NativeRuntime, value: Value) -> Option<Value> {
    if value.is_number() {
        return Some(value);
    }
    runtime
        .string_bytes(value)
        .and_then(parse_standard_number)
        .and_then(|number| number.to_float().map(Value::float))
}

fn random_float(value: u64) -> LuaFloat {
    const SCALE: LuaFloat = 1.0 / ((1_u64 << 53) as LuaFloat);
    ((value >> 11) as LuaFloat) * SCALE
}

fn project_random(
    runtime: &mut dyn NativeRuntime,
    mut random: u64,
    limit: u64,
) -> Result<u64, NativeError> {
    let mut mask = limit;
    let mut shift = 1;
    while (mask & mask.wrapping_add(1)) != 0 {
        mask |= mask >> shift;
        shift *= 2;
    }
    loop {
        random &= mask;
        if random <= limit {
            return Ok(random);
        }
        random = runtime.next_random_u64()?;
    }
}

#[derive(Clone, Copy)]
enum Extrema {
    Min,
    Max,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ExtremaKind {
    Number,
    String,
}

fn extrema_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    extrema: Extrema,
) -> Result<Value, NativeError> {
    if args.is_empty() {
        return Err(NativeErrorKind::MissingArgument { index: 1 }.into());
    }

    let mut selected = comparable_arg(runtime, args, 1)?;
    for index in 2..=args.len() {
        let candidate = comparable_arg(runtime, args, index)?;
        let replace = match extrema {
            Extrema::Min => extrema_less(runtime, candidate, index, selected, 1)?,
            Extrema::Max => extrema_less(runtime, selected, 1, candidate, index)?,
        };
        if replace {
            selected = candidate;
        }
    }
    Ok(selected)
}

fn comparable_arg(
    runtime: &dyn NativeRuntime,
    args: &[Value],
    index: usize,
) -> Result<Value, NativeError> {
    let value = *args
        .get(index - 1)
        .ok_or(NativeErrorKind::MissingArgument { index })?;
    if extrema_kind(runtime, value).is_some() {
        Ok(value)
    } else {
        Err(NativeErrorKind::TypeError {
            index,
            expected: "number or string",
        }
        .into())
    }
}

fn extrema_kind(runtime: &dyn NativeRuntime, value: Value) -> Option<ExtremaKind> {
    if value.is_number() {
        Some(ExtremaKind::Number)
    } else if runtime.string_bytes(value).is_some() {
        Some(ExtremaKind::String)
    } else {
        None
    }
}

fn extrema_less(
    runtime: &dyn NativeRuntime,
    left: Value,
    left_index: usize,
    right: Value,
    right_index: usize,
) -> Result<bool, NativeError> {
    match (extrema_kind(runtime, left), extrema_kind(runtime, right)) {
        (Some(ExtremaKind::Number), Some(ExtremaKind::Number)) => {
            let left_float = left.to_float().expect("number values convert to float");
            let right_float = right.to_float().expect("number values convert to float");
            Ok(left_float < right_float)
        }
        (Some(ExtremaKind::String), Some(ExtremaKind::String)) => {
            let left_bytes = runtime
                .string_bytes(left)
                .expect("string kind has runtime bytes");
            let right_bytes = runtime
                .string_bytes(right)
                .expect("string kind has runtime bytes");
            Ok(left_bytes < right_bytes)
        }
        (Some(_), Some(_)) => Err(NativeErrorKind::RuntimeError {
            message: "attempt to compare values of different types".into(),
        }
        .into()),
        (None, _) => Err(NativeErrorKind::TypeError {
            index: left_index,
            expected: "number or string",
        }
        .into()),
        (_, None) => Err(NativeErrorKind::TypeError {
            index: right_index,
            expected: "number or string",
        }
        .into()),
    }
}

#[cfg(test)]
mod native_tests;

#[cfg(test)]
mod tests {
    use elara_core::{LuaInteger, Value};

    use super::{
        LuaRandomState, MATH_CONSTANTS, MATH_NATIVE_FUNCTIONS, PI, math_abs, math_ceil, math_floor,
        math_fmod, math_frexp, math_ldexp, math_max, math_min, math_random, math_randomseed,
        math_sqrt, math_tointeger, math_type, math_ult,
    };
    use crate::{FunctionSpec, NativeError, NativeErrorKind, NativeRuntime, StdLib};

    #[derive(Default)]
    struct TestRuntime {
        strings: Vec<Box<[u8]>>,
        random: LuaRandomState,
    }

    impl NativeRuntime for TestRuntime {
        fn intern_short_string(&mut self, bytes: &[u8]) -> Result<Value, NativeError> {
            let index = u32::try_from(self.strings.len()).expect("test string index fits in u32");
            self.strings.push(bytes.into());
            Ok(Value::table_index(index))
        }

        fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
            let index = value.as_table_index()? as usize;
            self.strings.get(index).map(Box::as_ref)
        }

        fn next_random_u64(&mut self) -> Result<u64, NativeError> {
            Ok(self.random.next_u64())
        }

        fn random_seed(&mut self) -> Result<u64, NativeError> {
            Ok(0x4567_89ab)
        }

        fn set_random_seed(&mut self, seed1: u64, seed2: u64) -> Result<(), NativeError> {
            self.random = LuaRandomState::from_seeds(seed1, seed2);
            Ok(())
        }
    }

    fn call(function: crate::NativeStdFunction, args: &[Value]) -> Vec<Value> {
        function(&mut TestRuntime::default(), args).expect("native should pass")
    }

    #[test]
    fn math_native_specs_cover_executable_subset() {
        let descriptors: Vec<_> = MATH_NATIVE_FUNCTIONS
            .iter()
            .map(|function| function.descriptor())
            .collect();

        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "abs")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "acos")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "asin")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "atan")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "ceil")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "cos")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "deg")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "exp")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "floor")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "fmod")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "frexp")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "ldexp")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "log")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "max")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "min")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "modf")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "random")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "randomseed")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "rad")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "sin")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "sqrt")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "tan")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "tointeger")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "type")));
        assert!(descriptors.contains(&FunctionSpec::new(StdLib::Math, "ult")));
    }

    #[test]
    fn math_constants_cover_standard_fields() {
        assert!(MATH_CONSTANTS.contains(&("pi", Value::float(PI))));
        assert!(MATH_CONSTANTS.contains(&("huge", Value::float(f64::INFINITY))));
        assert!(MATH_CONSTANTS.contains(&("maxinteger", Value::integer(i64::MAX))));
        assert!(MATH_CONSTANTS.contains(&("mininteger", Value::integer(i64::MIN))));
    }

    #[test]
    fn math_abs_preserves_integer_results() {
        assert_eq!(
            call(math_abs, &[Value::integer(-7)]),
            vec![Value::integer(7)]
        );
        assert_eq!(
            call(math_abs, &[Value::integer(LuaInteger::MIN)]),
            vec![Value::integer(LuaInteger::MIN)]
        );
    }

    #[test]
    fn math_abs_accepts_float_values() {
        assert_eq!(
            call(math_abs, &[Value::float(-2.5)]),
            vec![Value::float(2.5)]
        );
    }

    #[test]
    fn math_floor_and_ceil_preserve_or_convert_integer_results() {
        assert_eq!(
            call(math_floor, &[Value::integer(9)]),
            vec![Value::integer(9)]
        );
        assert_eq!(
            call(math_floor, &[Value::float(3.8)]),
            vec![Value::integer(3)]
        );
        assert_eq!(
            call(math_ceil, &[Value::float(3.2)]),
            vec![Value::integer(4)]
        );
    }

    #[test]
    fn math_sqrt_returns_float() {
        assert_eq!(
            call(math_sqrt, &[Value::integer(9)]),
            vec![Value::float(3.0)]
        );
    }

    #[test]
    fn math_number_arguments_coerce_numeric_strings_as_floats() {
        let mut runtime = TestRuntime::default();
        let three = runtime
            .intern_short_string(b"3")
            .expect("test string should intern");
        let seven = runtime
            .intern_short_string(b"7")
            .expect("test string should intern");
        let two = runtime
            .intern_short_string(b"2")
            .expect("test string should intern");
        let nine = runtime
            .intern_short_string(b"9")
            .expect("test string should intern");

        assert_eq!(
            math_abs(&mut runtime, &[three]),
            Ok(vec![Value::float(3.0)])
        );
        assert_eq!(
            math_fmod(&mut runtime, &[seven, two]),
            Ok(vec![Value::float(1.0)])
        );
        assert_eq!(
            math_sqrt(&mut runtime, &[nine]),
            Ok(vec![Value::float(3.0)])
        );
    }

    #[test]
    fn math_min_and_max_return_selected_original_value() {
        assert_eq!(
            call(
                math_min,
                &[Value::integer(3), Value::float(1.5), Value::integer(2)]
            ),
            vec![Value::float(1.5)]
        );
        assert_eq!(
            call(
                math_max,
                &[Value::float(3.5), Value::integer(7), Value::float(7.0)]
            ),
            vec![Value::integer(7)]
        );
    }

    #[test]
    fn math_min_and_max_compare_string_values() {
        let mut runtime = TestRuntime::default();
        let b = runtime
            .intern_short_string(b"b")
            .expect("test string should intern");
        let a = runtime
            .intern_short_string(b"a")
            .expect("test string should intern");
        let c = runtime
            .intern_short_string(b"c")
            .expect("test string should intern");

        assert_eq!(math_min(&mut runtime, &[b, a, c]), Ok(vec![a]));
        assert_eq!(math_max(&mut runtime, &[b, a, c]), Ok(vec![c]));
    }

    #[test]
    fn math_min_and_max_reject_mixed_number_and_string_values() {
        let mut runtime = TestRuntime::default();
        let text = runtime
            .intern_short_string(b"7")
            .expect("test string should intern");

        assert_eq!(
            math_min(&mut runtime, &[Value::integer(1), text])
                .expect_err("mixed comparison should fail")
                .kind(),
            &NativeErrorKind::RuntimeError {
                message: "attempt to compare values of different types".into()
            }
        );
    }

    #[test]
    fn math_frexp_and_ldexp_split_and_recombine_float() {
        assert_eq!(
            call(math_frexp, &[Value::float(12.0)]),
            vec![Value::float(0.75), Value::integer(4)]
        );
        assert_eq!(
            call(math_ldexp, &[Value::float(0.75), Value::integer(4)]),
            vec![Value::float(12.0)]
        );
    }

    #[test]
    fn math_random_without_bounds_returns_float_unit_interval() {
        let values = call(math_random, &[]);

        let value = values[0].as_float().expect("random should return a float");
        assert!((0.0..1.0).contains(&value));
    }

    #[test]
    fn math_random_returns_integer_inside_bounds() {
        let values = call(math_random, &[Value::integer(3), Value::integer(5)]);

        let value = values[0]
            .as_integer()
            .expect("random should return integer");
        assert!((3..=5).contains(&value));
    }

    #[test]
    fn math_random_zero_returns_full_integer() {
        let values = call(math_random, &[Value::integer(0)]);

        assert!(values[0].as_integer().is_some());
    }

    #[test]
    fn math_random_rejects_bad_arguments() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            math_random(&mut runtime, &[Value::integer(5), Value::integer(3)])
                .expect_err("empty interval should fail")
                .kind(),
            &NativeErrorKind::ArgumentOutOfRange { index: 1 }
        );
        assert_eq!(
            math_random(&mut runtime, &[Value::nil()])
                .expect_err("non-integer argument should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "integer"
            }
        );
    }

    #[test]
    fn math_integer_arguments_coerce_exact_numeric_strings() {
        let mut runtime = TestRuntime::default();
        let one = runtime
            .intern_short_string(b"1")
            .expect("test string should intern");
        let three = runtime
            .intern_short_string(b"3")
            .expect("test string should intern");
        let seed = runtime
            .intern_short_string(b"123")
            .expect("test string should intern");
        let hex_seed = runtime
            .intern_short_string(b"0x4")
            .expect("test string should intern");
        let fraction = runtime
            .intern_short_string(b"1.5")
            .expect("test string should intern");

        let random = math_random(&mut runtime, &[one, three]).expect("random should pass");
        let random = random[0]
            .as_integer()
            .expect("random should return integer");
        assert!((1..=3).contains(&random));
        assert_eq!(
            math_randomseed(&mut runtime, &[seed, hex_seed]),
            Ok(vec![Value::integer(123), Value::integer(4)])
        );
        assert_eq!(
            math_ult(&mut runtime, &[one, hex_seed]),
            Ok(vec![Value::boolean(true)])
        );
        assert_eq!(
            math_random(&mut runtime, &[fraction])
                .expect_err("non-integral string should fail")
                .kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "integer"
            }
        );
    }

    #[test]
    fn math_randomseed_returns_seeds_and_resets_sequence() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            math_randomseed(&mut runtime, &[Value::integer(7), Value::integer(9)]),
            Ok(vec![Value::integer(7), Value::integer(9)])
        );
        let first = math_random(&mut runtime, &[Value::integer(0)]).expect("random should pass");
        math_randomseed(&mut runtime, &[Value::integer(7), Value::integer(9)])
            .expect("randomseed should pass");

        assert_eq!(
            math_random(&mut runtime, &[Value::integer(0)]).expect("random should pass"),
            first
        );
    }

    #[test]
    fn math_randomseed_ignores_extra_arguments() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            math_randomseed(
                &mut runtime,
                &[Value::integer(7), Value::integer(9), Value::boolean(false)]
            ),
            Ok(vec![Value::integer(7), Value::integer(9)])
        );
        assert_eq!(
            math_randomseed(
                &mut runtime,
                &[Value::integer(7), Value::boolean(false), Value::integer(9)]
            )
            .expect_err("bad second seed should still fail")
            .kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "integer"
            }
        );
    }

    #[test]
    fn math_randomseed_without_arguments_uses_runtime_seed() {
        let mut runtime = TestRuntime::default();

        assert_eq!(
            math_randomseed(&mut runtime, &[]),
            Ok(vec![
                Value::integer(0x4567_89ab),
                Value::integer(270_385_360_242_450_737)
            ])
        );
    }

    #[test]
    fn math_natives_report_argument_errors() {
        let mut runtime = TestRuntime::default();
        let error = math_abs(&mut runtime, &[]).expect_err("missing argument should fail");
        assert_eq!(error.kind(), &NativeErrorKind::MissingArgument { index: 1 });

        let error =
            math_abs(&mut runtime, &[Value::nil()]).expect_err("non-number argument should fail");
        assert_eq!(
            error.kind(),
            &NativeErrorKind::TypeError {
                index: 1,
                expected: "number"
            }
        );

        let error = math_min(&mut runtime, &[Value::integer(1), Value::nil()])
            .expect_err("non-number arg should fail");
        assert_eq!(
            error.kind(),
            &NativeErrorKind::TypeError {
                index: 2,
                expected: "number or string"
            }
        );
    }

    #[test]
    fn math_type_reports_numeric_subtype() {
        let mut runtime = TestRuntime::default();

        let integer = math_type(&mut runtime, &[Value::integer(7)]).expect("type should pass");
        let float = math_type(&mut runtime, &[Value::float(7.0)]).expect("type should pass");

        assert_eq!(
            runtime.short_string_bytes(integer[0]),
            Some(b"integer".as_slice())
        );
        assert_eq!(
            runtime.short_string_bytes(float[0]),
            Some(b"float".as_slice())
        );
    }

    #[test]
    fn math_type_returns_nil_for_non_numbers() {
        assert_eq!(
            call(math_type, &[Value::boolean(false)]),
            vec![Value::nil()]
        );
    }

    #[test]
    fn math_tointeger_accepts_integral_numbers_and_strings() {
        let mut runtime = TestRuntime::default();
        let string = runtime
            .intern_short_string(b"  7.0  ")
            .expect("test string should intern");

        assert_eq!(
            math_tointeger(&mut runtime, &[Value::integer(7)]),
            Ok(vec![Value::integer(7)])
        );
        assert_eq!(
            math_tointeger(&mut runtime, &[Value::float(7.0)]),
            Ok(vec![Value::integer(7)])
        );
        assert_eq!(
            math_tointeger(&mut runtime, &[string]),
            Ok(vec![Value::integer(7)])
        );
    }

    #[test]
    fn math_tointeger_returns_nil_for_non_integral_or_non_numeric_values() {
        let mut runtime = TestRuntime::default();
        let string = runtime
            .intern_short_string(b"7.5")
            .expect("test string should intern");

        assert_eq!(
            math_tointeger(&mut runtime, &[Value::float(7.5)]),
            Ok(vec![Value::nil()])
        );
        assert_eq!(
            math_tointeger(&mut runtime, &[string]),
            Ok(vec![Value::nil()])
        );
        assert_eq!(
            math_tointeger(&mut runtime, &[Value::boolean(false)]),
            Ok(vec![Value::nil()])
        );
    }
}
