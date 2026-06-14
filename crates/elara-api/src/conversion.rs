//! Typed conversion helpers for the public API.

/// Owned Lua value used by high-level API conversions.
#[derive(Clone, Debug, PartialEq)]
pub enum LuaValue {
    /// Lua nil.
    Nil,
    /// Lua boolean.
    Boolean(bool),
    /// Lua integer.
    Integer(i64),
    /// Lua floating-point number.
    Number(f64),
    /// Lua string represented as owned UTF-8 text for the initial API.
    String(Box<str>),
}

impl LuaValue {
    /// Lua type name for diagnostics.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Boolean(_) => "boolean",
            Self::Integer(_) | Self::Number(_) => "number",
            Self::String(_) => "string",
        }
    }
}

/// Error raised by typed Lua value conversions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionError {
    expected: &'static str,
    actual: &'static str,
}

impl ConversionError {
    fn new(expected: &'static str, actual: &'static str) -> Self {
        Self { expected, actual }
    }

    /// Expected Lua type.
    #[must_use]
    pub const fn expected(&self) -> &'static str {
        self.expected
    }

    /// Actual Lua type.
    #[must_use]
    pub const fn actual(&self) -> &'static str {
        self.actual
    }
}

/// Converts a Rust value into an owned Lua API value.
pub trait IntoLua {
    /// Converts this value.
    fn into_lua(self) -> Result<LuaValue, ConversionError>;
}

/// Converts an owned Lua API value into a Rust value.
pub trait FromLua: Sized {
    /// Converts from one Lua value.
    fn from_lua(value: &LuaValue) -> Result<Self, ConversionError>;
}

/// Converts Rust values into zero or more Lua API values.
pub trait IntoLuaMulti {
    /// Converts this value list.
    fn into_lua_multi(self) -> Result<Vec<LuaValue>, ConversionError>;
}

/// Converts zero or more Lua API values into Rust values.
pub trait FromLuaMulti: Sized {
    /// Converts from a value slice.
    fn from_lua_multi(values: &[LuaValue]) -> Result<Self, ConversionError>;
}

impl IntoLua for () {
    fn into_lua(self) -> Result<LuaValue, ConversionError> {
        Ok(LuaValue::Nil)
    }
}

impl FromLua for () {
    fn from_lua(value: &LuaValue) -> Result<Self, ConversionError> {
        match value {
            LuaValue::Nil => Ok(()),
            value => Err(ConversionError::new("nil", value.type_name())),
        }
    }
}

impl IntoLua for bool {
    fn into_lua(self) -> Result<LuaValue, ConversionError> {
        Ok(LuaValue::Boolean(self))
    }
}

impl FromLua for bool {
    fn from_lua(value: &LuaValue) -> Result<Self, ConversionError> {
        match value {
            LuaValue::Boolean(value) => Ok(*value),
            value => Err(ConversionError::new("boolean", value.type_name())),
        }
    }
}

impl IntoLua for i64 {
    fn into_lua(self) -> Result<LuaValue, ConversionError> {
        Ok(LuaValue::Integer(self))
    }
}

impl FromLua for i64 {
    fn from_lua(value: &LuaValue) -> Result<Self, ConversionError> {
        match value {
            LuaValue::Integer(value) => Ok(*value),
            value => Err(ConversionError::new("integer", value.type_name())),
        }
    }
}

impl IntoLua for f64 {
    fn into_lua(self) -> Result<LuaValue, ConversionError> {
        Ok(LuaValue::Number(self))
    }
}

impl FromLua for f64 {
    fn from_lua(value: &LuaValue) -> Result<Self, ConversionError> {
        match value {
            LuaValue::Integer(value) => Ok(*value as f64),
            LuaValue::Number(value) => Ok(*value),
            value => Err(ConversionError::new("number", value.type_name())),
        }
    }
}

impl IntoLua for String {
    fn into_lua(self) -> Result<LuaValue, ConversionError> {
        Ok(LuaValue::String(self.into_boxed_str()))
    }
}

impl IntoLua for &str {
    fn into_lua(self) -> Result<LuaValue, ConversionError> {
        Ok(LuaValue::String(self.into()))
    }
}

impl FromLua for String {
    fn from_lua(value: &LuaValue) -> Result<Self, ConversionError> {
        match value {
            LuaValue::String(value) => Ok(value.to_string()),
            value => Err(ConversionError::new("string", value.type_name())),
        }
    }
}

impl<T> IntoLua for Option<T>
where
    T: IntoLua,
{
    fn into_lua(self) -> Result<LuaValue, ConversionError> {
        match self {
            Some(value) => value.into_lua(),
            None => Ok(LuaValue::Nil),
        }
    }
}

impl<T> FromLua for Option<T>
where
    T: FromLua,
{
    fn from_lua(value: &LuaValue) -> Result<Self, ConversionError> {
        match value {
            LuaValue::Nil => Ok(None),
            value => T::from_lua(value).map(Some),
        }
    }
}

impl IntoLuaMulti for () {
    fn into_lua_multi(self) -> Result<Vec<LuaValue>, ConversionError> {
        Ok(Vec::new())
    }
}

impl<T> IntoLuaMulti for (T,)
where
    T: IntoLua,
{
    fn into_lua_multi(self) -> Result<Vec<LuaValue>, ConversionError> {
        Ok(vec![self.0.into_lua()?])
    }
}

impl<A, B> IntoLuaMulti for (A, B)
where
    A: IntoLua,
    B: IntoLua,
{
    fn into_lua_multi(self) -> Result<Vec<LuaValue>, ConversionError> {
        Ok(vec![self.0.into_lua()?, self.1.into_lua()?])
    }
}

impl FromLuaMulti for () {
    fn from_lua_multi(_values: &[LuaValue]) -> Result<Self, ConversionError> {
        Ok(())
    }
}

impl<T> FromLuaMulti for (T,)
where
    T: FromLua,
{
    fn from_lua_multi(values: &[LuaValue]) -> Result<Self, ConversionError> {
        Ok((T::from_lua(values.first().unwrap_or(&LuaValue::Nil))?,))
    }
}

impl<A, B> FromLuaMulti for (A, B)
where
    A: FromLua,
    B: FromLua,
{
    fn from_lua_multi(values: &[LuaValue]) -> Result<Self, ConversionError> {
        let first = values.first().unwrap_or(&LuaValue::Nil);
        let second = values.get(1).unwrap_or(&LuaValue::Nil);
        Ok((A::from_lua(first)?, B::from_lua(second)?))
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversionError, FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, LuaValue};

    #[test]
    fn conversion_converts_primitives_into_lua_values() {
        assert_eq!(().into_lua(), Ok(LuaValue::Nil));
        assert_eq!(true.into_lua(), Ok(LuaValue::Boolean(true)));
        assert_eq!(42_i64.into_lua(), Ok(LuaValue::Integer(42)));
        assert_eq!(3.5_f64.into_lua(), Ok(LuaValue::Number(3.5)));
    }

    #[test]
    fn conversion_converts_strings_into_lua_values() {
        assert_eq!("hello".into_lua(), Ok(LuaValue::String("hello".into())));
        assert_eq!(
            String::from("owned").into_lua(),
            Ok(LuaValue::String("owned".into()))
        );
    }

    #[test]
    fn conversion_extracts_rust_values_from_lua_values() {
        assert_eq!(bool::from_lua(&LuaValue::Boolean(false)), Ok(false));
        assert_eq!(i64::from_lua(&LuaValue::Integer(7)), Ok(7));
        assert_eq!(f64::from_lua(&LuaValue::Integer(7)), Ok(7.0));
        assert_eq!(f64::from_lua(&LuaValue::Number(1.25)), Ok(1.25));
        assert_eq!(
            String::from_lua(&LuaValue::String("text".into())),
            Ok(String::from("text"))
        );
    }

    #[test]
    fn conversion_reports_type_mismatch() {
        assert_eq!(
            bool::from_lua(&LuaValue::String("no".into())),
            Err(ConversionError::new("boolean", "string"))
        );
    }

    #[test]
    fn conversion_handles_options() {
        assert_eq!(Option::<i64>::None.into_lua(), Ok(LuaValue::Nil));
        assert_eq!(Some(9_i64).into_lua(), Ok(LuaValue::Integer(9)));
        assert_eq!(Option::<i64>::from_lua(&LuaValue::Nil), Ok(None));
        assert_eq!(Option::<i64>::from_lua(&LuaValue::Integer(9)), Ok(Some(9)));
    }

    #[test]
    fn conversion_handles_tuple_multi_values() {
        assert_eq!(
            (1_i64, "two").into_lua_multi(),
            Ok(vec![LuaValue::Integer(1), LuaValue::String("two".into())])
        );
        assert_eq!(
            <(i64, String)>::from_lua_multi(&[
                LuaValue::Integer(1),
                LuaValue::String("two".into())
            ]),
            Ok((1, String::from("two")))
        );
    }
}
