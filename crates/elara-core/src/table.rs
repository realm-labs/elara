//! Lua table storage.

use crate::{GcHeader, GcKind, GcObject, LuaInteger, Value};

/// Lua table with array storage.
#[derive(Debug)]
pub struct Table {
    header: GcHeader,
    array: Vec<Value>,
    version: u32,
}

impl Table {
    /// Creates an empty table.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            header: GcHeader::new(GcKind::Table),
            array: Vec::new(),
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

#[cfg(test)]
mod tests {
    use crate::{GcKind, GcObject, Table, Value};

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
}
