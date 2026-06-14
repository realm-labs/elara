//! Safe high-level API handles.

use std::{any::Any, cell::RefCell, collections::BTreeMap, fmt, sync::Arc};

use crate::{ConversionError, FromLua, IntoLua, LuaValue};

/// Error raised by API registry operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    /// Registry key is not present in this Lua handle.
    MissingKey,
    /// Registry value could not be converted to the requested Rust type.
    Conversion(ConversionError),
}

impl From<ConversionError> for RegistryError {
    fn from(error: ConversionError) -> Self {
        Self::Conversion(error)
    }
}

/// Opaque key for a value stored in a Lua registry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RegistryKey(pub(crate) u64);

/// Owned table handle for high-level API values.
#[derive(Clone, Default)]
pub struct Table {
    entries: Arc<RefCell<BTreeMap<Box<str>, LuaValue>>>,
}

impl Table {
    /// Creates an empty table handle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a string-keyed field.
    pub fn set<V>(&self, key: impl Into<Box<str>>, value: V) -> Result<(), ConversionError>
    where
        V: IntoLua,
    {
        self.entries
            .borrow_mut()
            .insert(key.into(), value.into_lua()?);
        Ok(())
    }

    /// Gets a string-keyed field and converts it to a Rust value.
    pub fn get<T>(&self, key: &str) -> Result<T, ConversionError>
    where
        T: FromLua,
    {
        let entries = self.entries.borrow();
        T::from_lua(entries.get(key).unwrap_or(&LuaValue::Nil))
    }

    /// Returns whether the table contains a string key.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.borrow().contains_key(key)
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Table")
            .field("len", &self.entries.borrow().len())
            .finish()
    }
}

/// Trait for Rust values that can be stored as userdata.
pub trait UserData: Any + Send + Sync + 'static {}

impl<T> UserData for T where T: Any + Send + Sync + 'static {}

/// Type-erased userdata handle.
#[derive(Clone)]
pub struct AnyUserData {
    inner: Arc<dyn Any + Send + Sync>,
}

impl AnyUserData {
    /// Creates a userdata handle from a Rust value.
    #[must_use]
    pub fn new<T>(value: T) -> Self
    where
        T: UserData,
    {
        Self {
            inner: Arc::new(value),
        }
    }

    /// Returns true when this userdata stores `T`.
    #[must_use]
    pub fn is<T>(&self) -> bool
    where
        T: UserData,
    {
        self.inner.is::<T>()
    }

    /// Borrows this userdata as `T` when the stored type matches.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: UserData,
    {
        self.inner.downcast_ref::<T>()
    }
}

impl fmt::Debug for AnyUserData {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnyUserData")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use crate::{IntoLua, Lua, RegistryError, Table};

    #[test]
    fn table_handle_stores_and_converts_fields() {
        let table = Table::new();

        table.set("answer", 42_i64).expect("set should pass");
        table.set("name", "elara").expect("set should pass");

        assert!(table.contains_key("answer"));
        assert_eq!(table.get::<i64>("answer"), Ok(42));
        assert_eq!(table.get::<String>("name"), Ok(String::from("elara")));
    }

    #[test]
    fn registry_key_round_trips_values() {
        let lua = Lua::new();
        let key = lua
            .create_registry_value("stored")
            .expect("registry insert should pass");

        assert_eq!(
            lua.registry_value::<String>(&key),
            Ok(String::from("stored"))
        );
        assert_eq!(
            lua.remove_registry_value(key),
            Some("stored".into_lua().unwrap())
        );
        assert_eq!(
            lua.registry_value::<String>(&key),
            Err(RegistryError::MissingKey)
        );
    }

    #[test]
    fn userdata_handle_exposes_typed_borrow() {
        #[derive(Debug, Eq, PartialEq)]
        struct HostValue(i64);

        let lua = Lua::new();
        let userdata = lua.create_userdata(HostValue(7));

        assert!(userdata.is::<HostValue>());
        assert_eq!(userdata.downcast_ref::<HostValue>(), Some(&HostValue(7)));
        assert!(!userdata.is::<String>());
    }
}
