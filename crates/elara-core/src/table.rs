//! Lua table storage.

use std::collections::HashMap;

use crate::{GcHeader, GcKind, GcObject, GcRef, LuaFloat, LuaInteger, ShortString, Value};

/// Lua table with array storage.
#[derive(Debug)]
pub struct Table {
    header: GcHeader,
    array: Vec<Value>,
    hash: HashMap<TableKey, Value>,
    version: u32,
}

impl Table {
    /// Creates an empty table.
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: GcHeader::new(GcKind::Table),
            array: Vec::new(),
            hash: HashMap::new(),
            version: 0,
        }
    }

    /// Number of currently stored array slots.
    #[must_use]
    pub fn array_len(&self) -> usize {
        self.array.len()
    }

    /// Returns true when the array part has no slots.
    #[must_use]
    pub fn is_array_empty(&self) -> bool {
        self.array.is_empty()
    }

    /// Number of entries in the hash part.
    #[must_use]
    pub fn hash_len(&self) -> usize {
        self.hash.len()
    }

    /// Current capacity of the hash part.
    #[must_use]
    pub fn hash_capacity(&self) -> usize {
        self.hash.capacity()
    }

    /// Table version, incremented on raw array mutations.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Gets a value from the array part using a Lua integer key.
    #[must_use]
    pub fn raw_get_integer(&self, index: LuaInteger) -> Value {
        let Some(offset) = array_offset(index) else {
            return Value::nil();
        };

        self.array.get(offset).copied().unwrap_or_else(Value::nil)
    }

    /// Sets a value in the array part using a Lua integer key.
    ///
    /// Returns false when the key cannot be represented as a valid array index
    /// or when growing the array allocation fails.
    pub fn raw_set_integer(&mut self, index: LuaInteger, value: Value) -> bool {
        let Some(offset) = array_offset(index) else {
            return false;
        };

        if value.is_nil() {
            if offset < self.array.len() {
                self.array[offset] = Value::nil();
                self.trim_trailing_nil();
                self.bump_version();
            }
            return true;
        }

        let Some(new_len) = offset.checked_add(1) else {
            return false;
        };

        if new_len > self.array.len() {
            let additional = new_len - self.array.len();
            if self.array.try_reserve(additional).is_err() {
                return false;
            }
            self.array.resize(new_len, Value::nil());
        }

        self.array[offset] = value;
        self.bump_version();
        true
    }

    /// Gets a value using a Lua value key.
    #[must_use]
    pub fn raw_get_value(&self, key: Value) -> Value {
        let Some(key) = TableKey::from_value(key) else {
            return Value::nil();
        };

        if let TableKey::Integer(index) = key
            && index >= 1
        {
            return self.raw_get_integer(index);
        }

        self.hash.get(&key).copied().unwrap_or_else(Value::nil)
    }

    /// Sets a value using a Lua value key.
    ///
    /// Returns false for invalid Lua table keys such as nil and NaN.
    pub fn raw_set_value(&mut self, key: Value, value: Value) -> bool {
        let Some(key) = TableKey::from_value(key) else {
            return false;
        };

        if let TableKey::Integer(index) = key
            && index >= 1
        {
            return self.raw_set_integer(index, value);
        }

        if value.is_nil() {
            if self.hash.remove(&key).is_some() {
                self.bump_version();
            }
            return true;
        }

        let previous = self.hash.insert(key, value);
        if previous != Some(value) {
            self.bump_version();
        }
        true
    }

    fn trim_trailing_nil(&mut self) {
        while self.array.last().is_some_and(|value| value.is_nil()) {
            self.array.pop();
        }
    }

    fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl GcObject for Table {
    fn header(&self) -> &GcHeader {
        &self.header
    }
}

fn array_offset(index: LuaInteger) -> Option<usize> {
    if index < 1 {
        return None;
    }

    usize::try_from(index - 1).ok()
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TableKey {
    Bool(bool),
    Integer(LuaInteger),
    Float(u64),
    ShortString(GcRef<ShortString>),
}

impl TableKey {
    fn from_value(value: Value) -> Option<Self> {
        if let Some(value) = value.as_bool() {
            return Some(Self::Bool(value));
        }
        if let Some(value) = value.as_integer() {
            return Some(Self::Integer(value));
        }
        if let Some(value) = value.as_float() {
            return Self::from_float(value);
        }
        if let Some(value) = value.as_short_string() {
            return Some(Self::ShortString(value));
        }

        None
    }

    fn from_float(value: LuaFloat) -> Option<Self> {
        if value.is_nan() {
            return None;
        }

        if let Some(integer) = crate::float_to_integer_exact(value) {
            return Some(Self::Integer(integer));
        }

        let normalized = if value == 0.0 { 0.0 } else { value };
        Some(Self::Float(normalized.to_bits()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{GcArena, GcKind, GcObject, StringInterner, Table, Value, float_to_integer_exact};

    #[test]
    fn table_array_new_table_is_empty() {
        let table = Table::new();

        assert_eq!(table.header().kind(), GcKind::Table);
        assert_eq!(table.array_len(), 0);
        assert!(table.is_array_empty());
        assert_eq!(table.version(), 0);
        assert_eq!(table.raw_get_integer(1), Value::nil());
        assert_eq!(table.raw_get_integer(0), Value::nil());
    }

    #[test]
    fn table_array_set_and_get_integer_indices() {
        let mut table = Table::new();

        assert!(table.raw_set_integer(1, Value::integer(42)));

        assert_eq!(table.array_len(), 1);
        assert_eq!(table.version(), 1);
        assert_eq!(table.raw_get_integer(1), Value::integer(42));
        assert_eq!(table.raw_get_integer(2), Value::nil());
    }

    #[test]
    fn table_array_grows_and_fills_holes_with_nil() {
        let mut table = Table::new();

        assert!(table.raw_set_integer(3, Value::boolean(true)));

        assert_eq!(table.array_len(), 3);
        assert_eq!(table.raw_get_integer(1), Value::nil());
        assert_eq!(table.raw_get_integer(2), Value::nil());
        assert_eq!(table.raw_get_integer(3), Value::boolean(true));
    }

    #[test]
    fn table_array_nil_assignment_clears_slots_and_trims_tail() {
        let mut table = Table::new();

        assert!(table.raw_set_integer(1, Value::integer(1)));
        assert!(table.raw_set_integer(2, Value::integer(2)));
        assert!(table.raw_set_integer(3, Value::integer(3)));
        assert_eq!(table.array_len(), 3);

        assert!(table.raw_set_integer(2, Value::nil()));
        assert_eq!(table.array_len(), 3);
        assert_eq!(table.raw_get_integer(2), Value::nil());

        assert!(table.raw_set_integer(3, Value::nil()));
        assert_eq!(table.array_len(), 1);
        assert_eq!(table.raw_get_integer(1), Value::integer(1));
    }

    #[test]
    fn table_array_nil_assignment_does_not_grow() {
        let mut table = Table::new();

        assert!(table.raw_set_integer(4, Value::nil()));

        assert_eq!(table.array_len(), 0);
        assert_eq!(table.version(), 0);
    }

    #[test]
    fn table_array_rejects_non_positive_indices() {
        let mut table = Table::new();

        assert!(!table.raw_set_integer(0, Value::integer(1)));
        assert!(!table.raw_set_integer(-1, Value::integer(1)));
        assert_eq!(table.raw_get_integer(-1), Value::nil());
        assert_eq!(table.array_len(), 0);
    }

    #[test]
    fn table_hash_stores_boolean_keys() {
        let mut table = Table::new();

        assert!(table.raw_set_value(Value::boolean(true), Value::integer(1)));
        assert!(table.raw_set_value(Value::boolean(false), Value::integer(2)));

        assert_eq!(table.hash_len(), 2);
        assert_eq!(table.raw_get_value(Value::boolean(true)), Value::integer(1));
        assert_eq!(
            table.raw_get_value(Value::boolean(false)),
            Value::integer(2)
        );
    }

    #[test]
    fn table_hash_canonicalizes_integer_like_float_keys() {
        let mut table = Table::new();

        assert!(table.raw_set_value(Value::integer(0), Value::integer(10)));

        assert_eq!(float_to_integer_exact(0.0), Some(0));
        assert_eq!(table.raw_get_value(Value::float(0.0)), Value::integer(10));

        assert!(table.raw_set_value(Value::float(-0.0), Value::integer(11)));
        assert_eq!(table.raw_get_value(Value::integer(0)), Value::integer(11));
        assert_eq!(table.hash_len(), 1);
    }

    #[test]
    fn table_hash_stores_non_integral_float_keys() {
        let mut table = Table::new();

        assert!(table.raw_set_value(Value::float(1.5), Value::integer(15)));

        assert_eq!(table.hash_len(), 1);
        assert_eq!(table.raw_get_value(Value::float(1.5)), Value::integer(15));
        assert_eq!(table.raw_get_value(Value::float(1.25)), Value::nil());
    }

    #[test]
    fn table_hash_rejects_nil_and_nan_keys() {
        let mut table = Table::new();

        assert!(!table.raw_set_value(Value::nil(), Value::integer(1)));
        assert!(!table.raw_set_value(Value::float(f64::NAN), Value::integer(1)));
        assert_eq!(table.raw_get_value(Value::float(f64::NAN)), Value::nil());
        assert_eq!(table.hash_len(), 0);
    }

    #[test]
    fn table_hash_stores_short_string_keys() {
        let mut arena = GcArena::new();
        let mut interner = StringInterner::new();
        let key = interner.intern_short(&mut arena, "name");
        let same_key = interner.intern_short(&mut arena, b"name");
        let mut table = Table::new();

        assert!(table.raw_set_value(Value::short_string(key), Value::integer(7)));

        assert_eq!(table.hash_len(), 1);
        assert_eq!(
            table.raw_get_value(Value::short_string(same_key)),
            Value::integer(7)
        );
    }

    #[test]
    fn table_hash_nil_assignment_removes_hash_entries() {
        let mut table = Table::new();

        assert!(table.raw_set_value(Value::boolean(true), Value::integer(1)));
        assert_eq!(table.hash_len(), 1);

        assert!(table.raw_set_value(Value::boolean(true), Value::nil()));

        assert_eq!(table.hash_len(), 0);
        assert_eq!(table.raw_get_value(Value::boolean(true)), Value::nil());
    }

    #[test]
    fn table_hash_grows_capacity_for_many_entries() {
        let mut table = Table::new();
        let initial_capacity = table.hash_capacity();

        for index in 1..64 {
            assert!(table.raw_set_value(Value::integer(-index), Value::integer(index)));
        }

        assert_eq!(table.hash_len(), 63);
        assert!(table.hash_capacity() > initial_capacity);
    }
}
