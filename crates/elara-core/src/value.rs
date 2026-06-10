//! Lua value primitives.

use crate::{GcRef, LongString, ShortString};

/// Integer type used by the current Lua target.
pub type LuaInteger = i64;

/// Floating-point type used by the current Lua target.
pub type LuaFloat = f64;

const I64_MAX_PLUS_ONE_AS_F64: LuaFloat = 9_223_372_036_854_775_808.0;

/// Public tag for a Lua value.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ValueTag {
    /// Lua `nil`.
    Nil,
    /// Lua boolean.
    Bool,
    /// Lua integer number.
    Integer,
    /// Lua floating-point number.
    Float,
    /// Interned short string.
    ShortString,
    /// Long string.
    LongString,
    /// Table object.
    Table,
    /// Lua, native, or C closure.
    Closure,
    /// Lua thread or coroutine.
    Thread,
    /// Full userdata.
    UserData,
    /// Light userdata.
    LightUserData,
}

/// Primitive Lua value.
#[derive(Clone, Copy, Debug)]
pub struct Value {
    repr: ValueRepr,
}

#[derive(Clone, Copy, Debug)]
enum ValueRepr {
    Nil,
    Bool(bool),
    Integer(LuaInteger),
    Float(LuaFloat),
    ShortString(GcRef<ShortString>),
    LongString(GcRef<LongString>),
    Table(u32),
    Closure(u32),
    NativeFunction(u32),
}

impl Value {
    /// Lua `nil`.
    pub const NIL: Self = Self {
        repr: ValueRepr::Nil,
    };

    /// Creates Lua `nil`.
    #[must_use]
    pub const fn nil() -> Self {
        Self::NIL
    }

    /// Creates a Lua boolean.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self {
            repr: ValueRepr::Bool(value),
        }
    }

    /// Creates a Lua integer.
    #[must_use]
    pub const fn integer(value: LuaInteger) -> Self {
        Self {
            repr: ValueRepr::Integer(value),
        }
    }

    /// Creates a Lua float.
    #[must_use]
    pub const fn float(value: LuaFloat) -> Self {
        Self {
            repr: ValueRepr::Float(value),
        }
    }

    /// Creates a Lua short string value.
    #[must_use]
    pub const fn short_string(value: GcRef<ShortString>) -> Self {
        Self {
            repr: ValueRepr::ShortString(value),
        }
    }

    /// Creates a Lua long string value.
    #[must_use]
    pub const fn long_string(value: GcRef<LongString>) -> Self {
        Self {
            repr: ValueRepr::LongString(value),
        }
    }

    /// Creates a temporary Lua table value from a runtime table index.
    #[must_use]
    pub const fn table_index(value: u32) -> Self {
        Self {
            repr: ValueRepr::Table(value),
        }
    }

    /// Creates a temporary Lua closure value from a child prototype index.
    #[must_use]
    pub const fn closure_index(value: u32) -> Self {
        Self {
            repr: ValueRepr::Closure(value),
        }
    }

    /// Creates a temporary Lua native-function value from a runtime native index.
    #[must_use]
    pub const fn native_function_index(value: u32) -> Self {
        Self {
            repr: ValueRepr::NativeFunction(value),
        }
    }

    /// Returns the value tag.
    #[must_use]
    pub const fn tag(self) -> ValueTag {
        match self.repr {
            ValueRepr::Nil => ValueTag::Nil,
            ValueRepr::Bool(_) => ValueTag::Bool,
            ValueRepr::Integer(_) => ValueTag::Integer,
            ValueRepr::Float(_) => ValueTag::Float,
            ValueRepr::ShortString(_) => ValueTag::ShortString,
            ValueRepr::LongString(_) => ValueTag::LongString,
            ValueRepr::Table(_) => ValueTag::Table,
            ValueRepr::Closure(_) | ValueRepr::NativeFunction(_) => ValueTag::Closure,
        }
    }

    /// Returns true for Lua `nil`.
    #[must_use]
    pub const fn is_nil(self) -> bool {
        matches!(self.repr, ValueRepr::Nil)
    }

    /// Returns true for Lua booleans.
    #[must_use]
    pub const fn is_bool(self) -> bool {
        matches!(self.repr, ValueRepr::Bool(_))
    }

    /// Returns true for Lua integers or floats.
    #[must_use]
    pub const fn is_number(self) -> bool {
        matches!(self.repr, ValueRepr::Integer(_) | ValueRepr::Float(_))
    }

    /// Returns true for Lua strings.
    #[must_use]
    pub const fn is_string(self) -> bool {
        matches!(
            self.repr,
            ValueRepr::ShortString(_) | ValueRepr::LongString(_)
        )
    }

    /// Returns true for Lua table placeholders.
    #[must_use]
    pub const fn is_table(self) -> bool {
        matches!(self.repr, ValueRepr::Table(_))
    }

    /// Returns true for Lua closure placeholders.
    #[must_use]
    pub const fn is_closure(self) -> bool {
        matches!(
            self.repr,
            ValueRepr::Closure(_) | ValueRepr::NativeFunction(_)
        )
    }

    /// Returns the boolean payload when this value is a boolean.
    #[must_use]
    pub const fn as_bool(self) -> Option<bool> {
        match self.repr {
            ValueRepr::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the integer payload when this value is an integer.
    #[must_use]
    pub const fn as_integer(self) -> Option<LuaInteger> {
        match self.repr {
            ValueRepr::Integer(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the float payload when this value is a float.
    #[must_use]
    pub const fn as_float(self) -> Option<LuaFloat> {
        match self.repr {
            ValueRepr::Float(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the short string payload when this value is a short string.
    #[must_use]
    pub const fn as_short_string(self) -> Option<GcRef<ShortString>> {
        match self.repr {
            ValueRepr::ShortString(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the long string payload when this value is a long string.
    #[must_use]
    pub const fn as_long_string(self) -> Option<GcRef<LongString>> {
        match self.repr {
            ValueRepr::LongString(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the runtime table index when this value is a table placeholder.
    #[must_use]
    pub const fn as_table_index(self) -> Option<u32> {
        match self.repr {
            ValueRepr::Table(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the child prototype index when this value is a closure placeholder.
    #[must_use]
    pub const fn as_closure_index(self) -> Option<u32> {
        match self.repr {
            ValueRepr::Closure(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the runtime native-function index when this value is a native function.
    #[must_use]
    pub const fn as_native_function_index(self) -> Option<u32> {
        match self.repr {
            ValueRepr::NativeFunction(value) => Some(value),
            _ => None,
        }
    }

    /// Converts an integer or float value to a float.
    #[must_use]
    pub fn to_float(self) -> Option<LuaFloat> {
        match self.repr {
            ValueRepr::Integer(value) => Some(value as LuaFloat),
            ValueRepr::Float(value) => Some(value),
            _ => None,
        }
    }

    /// Converts an integer or exactly integral float to an integer.
    #[must_use]
    pub fn to_integer_exact(self) -> Option<LuaInteger> {
        match self.repr {
            ValueRepr::Integer(value) => Some(value),
            ValueRepr::Float(value) => float_to_integer_exact(value),
            _ => None,
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::boolean(value)
    }
}

impl From<LuaInteger> for Value {
    fn from(value: LuaInteger) -> Self {
        Self::integer(value)
    }
}

impl From<LuaFloat> for Value {
    fn from(value: LuaFloat) -> Self {
        Self::float(value)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self.repr, other.repr) {
            (ValueRepr::Nil, ValueRepr::Nil) => true,
            (ValueRepr::Bool(left), ValueRepr::Bool(right)) => left == right,
            (ValueRepr::Integer(left), ValueRepr::Integer(right)) => left == right,
            (ValueRepr::Float(left), ValueRepr::Float(right)) => left == right,
            (ValueRepr::Integer(left), ValueRepr::Float(right)) => integer_float_eq(left, right),
            (ValueRepr::Float(left), ValueRepr::Integer(right)) => integer_float_eq(right, left),
            (ValueRepr::ShortString(left), ValueRepr::ShortString(right)) => left == right,
            (ValueRepr::LongString(left), ValueRepr::LongString(right)) => left == right,
            (ValueRepr::Table(left), ValueRepr::Table(right)) => left == right,
            (ValueRepr::Closure(left), ValueRepr::Closure(right)) => left == right,
            (ValueRepr::NativeFunction(left), ValueRepr::NativeFunction(right)) => left == right,
            _ => false,
        }
    }
}

/// Converts a float to an integer only when the float represents an exact Lua
/// integer value.
#[must_use]
pub fn float_to_integer_exact(value: LuaFloat) -> Option<LuaInteger> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < LuaInteger::MIN as LuaFloat
        || value >= I64_MAX_PLUS_ONE_AS_F64
    {
        return None;
    }

    Some(value as LuaInteger)
}

fn integer_float_eq(integer: LuaInteger, float: LuaFloat) -> bool {
    float_to_integer_exact(float) == Some(integer)
}

#[cfg(test)]
mod tests {
    use super::{LuaInteger, Value, ValueTag, float_to_integer_exact};

    #[test]
    fn value_nil_has_nil_tag() {
        let value = Value::nil();

        assert_eq!(value.tag(), ValueTag::Nil);
        assert!(value.is_nil());
        assert!(!value.is_bool());
        assert!(!value.is_number());
    }

    #[test]
    fn value_bool_round_trips() {
        let value = Value::boolean(true);

        assert_eq!(value.tag(), ValueTag::Bool);
        assert_eq!(value.as_bool(), Some(true));
        assert_eq!(Value::from(false).as_bool(), Some(false));
    }

    #[test]
    fn value_integer_round_trips() {
        let value = Value::integer(42);

        assert_eq!(value.tag(), ValueTag::Integer);
        assert_eq!(value.as_integer(), Some(42));
        assert_eq!(value.to_integer_exact(), Some(42));
        assert_eq!(value.to_float(), Some(42.0));
    }

    #[test]
    fn value_float_round_trips() {
        let value = Value::float(3.5);

        assert_eq!(value.tag(), ValueTag::Float);
        assert_eq!(value.as_float(), Some(3.5));
        assert_eq!(value.to_float(), Some(3.5));
        assert_eq!(value.to_integer_exact(), None);
    }

    #[test]
    fn value_table_placeholder_round_trips() {
        let value = Value::table_index(7);

        assert_eq!(value.tag(), ValueTag::Table);
        assert!(value.is_table());
        assert_eq!(value.as_table_index(), Some(7));
    }

    #[test]
    fn value_native_function_placeholder_round_trips() {
        let value = Value::native_function_index(3);

        assert_eq!(value.tag(), ValueTag::Closure);
        assert!(value.is_closure());
        assert_eq!(value.as_native_function_index(), Some(3));
        assert_eq!(value.as_closure_index(), None);
    }

    #[test]
    fn value_equality_matches_lua_primitive_number_rules() {
        assert_eq!(Value::nil(), Value::nil());
        assert_eq!(Value::boolean(true), Value::boolean(true));
        assert_ne!(Value::boolean(true), Value::boolean(false));
        assert_eq!(Value::integer(42), Value::integer(42));
        assert_eq!(Value::float(42.0), Value::float(42.0));
        assert_eq!(Value::integer(42), Value::float(42.0));
        assert_ne!(Value::integer(42), Value::float(42.5));
        assert_eq!(Value::table_index(1), Value::table_index(1));
        assert_ne!(Value::table_index(1), Value::table_index(2));
        assert_ne!(Value::integer(42), Value::boolean(true));
    }

    #[test]
    fn value_nan_is_not_equal_to_itself() {
        let value = Value::float(f64::NAN);

        assert_ne!(value, value);
        assert_eq!(value.to_integer_exact(), None);
    }

    #[test]
    fn value_float_to_integer_requires_exact_integral_range() {
        assert_eq!(float_to_integer_exact(0.0), Some(0));
        assert_eq!(float_to_integer_exact(-12.0), Some(-12));
        assert_eq!(float_to_integer_exact(12.25), None);
        assert_eq!(float_to_integer_exact(f64::INFINITY), None);
        assert_eq!(float_to_integer_exact(f64::NAN), None);
    }

    #[test]
    fn value_float_to_integer_rejects_out_of_range_upper_bound() {
        let too_large = 9_223_372_036_854_775_808.0;

        assert_eq!(float_to_integer_exact(too_large), None);
        assert_ne!(Value::integer(LuaInteger::MAX), Value::float(too_large));
    }

    #[test]
    fn value_float_to_integer_accepts_i64_min() {
        let min = LuaInteger::MIN as f64;

        assert_eq!(float_to_integer_exact(min), Some(LuaInteger::MIN));
        assert_eq!(Value::integer(LuaInteger::MIN), Value::float(min));
    }
}
