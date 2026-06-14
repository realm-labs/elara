//! Lua table storage.

use std::collections::HashMap;

use crate::{
    GcHeader, GcKind, GcObject, GcRef, GcTracer, GcWeakSweeper, LuaFloat, LuaInteger, ShortString,
    Value,
};

/// Placeholder cache flags for table metatable lookups.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetaFlags {
    bits: u16,
}

impl MetaFlags {
    /// Empty metadata cache flags.
    pub const EMPTY: Self = Self { bits: 0 };

    /// Creates empty metadata cache flags.
    #[must_use]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Raw flag bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Returns true when no metadata cache flags are set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    fn clear_missing_cache(&mut self) {
        self.bits = 0;
    }
}

/// Weak-reference mode for a table.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum WeakMode {
    /// Keys and values are strong.
    #[default]
    None,
    /// Collectable keys are weak and values are ephemeron-reachable from live keys.
    Keys,
    /// Values are weak.
    Values,
    /// Collectable keys and values are weak.
    KeysAndValues,
}

impl WeakMode {
    /// Returns true when collectable keys are weak.
    #[must_use]
    pub const fn weak_keys(self) -> bool {
        matches!(self, Self::Keys | Self::KeysAndValues)
    }

    /// Returns true when values are weak.
    #[must_use]
    pub const fn weak_values(self) -> bool {
        matches!(self, Self::Values | Self::KeysAndValues)
    }
}

/// Lua table with split array/hash storage and metadata.
#[derive(Debug)]
pub struct Table {
    header: GcHeader,
    array: Vec<Value>,
    hash: HashMap<TableKey, Value>,
    metatable: Option<GcRef<Table>>,
    weak_mode: WeakMode,
    flags: MetaFlags,
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
            metatable: None,
            weak_mode: WeakMode::None,
            flags: MetaFlags::empty(),
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

    /// Metatable cache flags.
    #[must_use]
    pub const fn flags(&self) -> MetaFlags {
        self.flags
    }

    /// Returns true when this table has a metatable.
    #[must_use]
    pub const fn has_metatable(&self) -> bool {
        self.metatable.is_some()
    }

    /// Table version, incremented on structural mutations.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Current metatable reference, if present.
    ///
    /// The returned reference is a low-level GC reference and does not root the
    /// metatable.
    #[must_use]
    pub const fn metatable(&self) -> Option<GcRef<Table>> {
        self.metatable
    }

    /// Current table weak-reference mode.
    #[must_use]
    pub const fn weak_mode(&self) -> WeakMode {
        self.weak_mode
    }

    /// Updates the table weak-reference mode.
    pub fn set_weak_mode(&mut self, weak_mode: WeakMode) {
        if self.weak_mode == weak_mode {
            return;
        }
        self.weak_mode = weak_mode;
        self.bump_version();
    }

    /// Updates the metatable reference.
    ///
    /// This invalidates metatable cache flags and bumps the table version only
    /// when the reference changes.
    pub fn set_metatable(&mut self, metatable: Option<GcRef<Table>>) {
        if self.metatable == metatable {
            return;
        }

        if let Some(metatable) = metatable {
            self.header.write_barrier_ref(metatable);
        }
        self.metatable = metatable;
        self.flags.clear_missing_cache();
        self.bump_version();
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
                let previous_len = self.array.len();
                let previous = self.array[offset];
                if previous.is_nil() && offset + 1 != previous_len {
                    return true;
                }

                self.array[offset] = Value::nil();
                self.trim_trailing_nil();
                if previous != Value::nil() || self.array.len() != previous_len {
                    self.bump_version();
                }
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

        if self.array[offset] == value {
            return true;
        }

        self.header.write_barrier_value(value);
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

        self.header.write_barrier_value(key.to_value());
        self.header.write_barrier_value(value);
        let previous = self.hash.insert(key, value);
        if previous != Some(value) {
            self.bump_version();
        }
        true
    }

    /// Returns the next raw key/value pair after `key`, or the first pair when
    /// `key` is nil.
    #[must_use]
    pub fn raw_next(&self, key: Value) -> Option<(Value, Value)> {
        let next_index = if key.is_nil() {
            Some(0)
        } else if let Some(index) = key.as_integer() {
            usize::try_from(index).ok()
        } else {
            None
        };

        if let Some(start) = next_index {
            if let Some((offset, value)) = self
                .array
                .iter()
                .copied()
                .enumerate()
                .skip(start)
                .find(|(_, value)| !value.is_nil())
            {
                let index =
                    LuaInteger::try_from(offset + 1).expect("array index fits in LuaInteger");
                return Some((Value::integer(index), value));
            }
            if key.is_nil() || key.as_integer().is_some() {
                return self.first_hash_pair();
            }
        }

        let key = TableKey::from_value(key)?;
        let mut found = false;
        for (entry_key, value) in &self.hash {
            if found && !value.is_nil() {
                return Some((entry_key.to_value(), *value));
            }
            if *entry_key == key {
                found = true;
            }
        }
        None
    }

    fn first_hash_pair(&self) -> Option<(Value, Value)> {
        self.hash
            .iter()
            .find(|(_, value)| !value.is_nil())
            .map(|(key, value)| (key.to_value(), *value))
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

    fn trace(&self, tracer: &mut GcTracer<'_>) {
        if let Some(metatable) = self.metatable {
            tracer.mark_ref(metatable);
        }

        if !self.weak_mode.weak_values() {
            for value in &self.array {
                tracer.mark_value(*value);
            }
        }

        for (key, value) in &self.hash {
            if !self.weak_mode.weak_keys() {
                key.trace(tracer);
            }

            if self.weak_mode.weak_values() {
                continue;
            }

            if self.weak_mode.weak_keys() {
                if key.is_collectable() {
                    tracer.mark_ephemeron(key.to_value(), *value);
                } else {
                    tracer.mark_value(*value);
                }
            } else {
                tracer.mark_value(*value);
            }
        }
    }

    fn remove_dead_weak_references(&mut self, sweeper: &GcWeakSweeper<'_>) {
        if self.weak_mode == WeakMode::None {
            return;
        }

        if self.weak_mode.weak_values() {
            for value in &mut self.array {
                if !sweeper.is_value_live(*value) {
                    *value = Value::nil();
                }
            }
            self.trim_trailing_nil();
        }

        self.hash.retain(|key, value| {
            let key_live = !self.weak_mode.weak_keys() || key.is_live(sweeper);
            let value_live = !self.weak_mode.weak_values() || sweeper.is_value_live(*value);
            key_live && value_live
        });
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
    fn trace(self, tracer: &mut GcTracer<'_>) {
        if let Self::ShortString(reference) = self {
            tracer.mark_ref(reference);
        }
    }

    fn is_collectable(self) -> bool {
        matches!(self, Self::ShortString(_))
    }

    fn is_live(self, sweeper: &GcWeakSweeper<'_>) -> bool {
        match self {
            Self::ShortString(reference) => sweeper.is_ref_live(reference),
            Self::Bool(_) | Self::Integer(_) | Self::Float(_) => true,
        }
    }

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

    fn to_value(self) -> Value {
        match self {
            Self::Bool(value) => Value::boolean(value),
            Self::Integer(value) => Value::integer(value),
            Self::Float(value) => Value::float(LuaFloat::from_bits(value)),
            Self::ShortString(value) => Value::short_string(value),
        }
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
    fn table_raw_next_traverses_array_entries() {
        let mut table = Table::new();
        assert!(table.raw_set_integer(1, Value::integer(10)));
        assert!(table.raw_set_integer(3, Value::integer(30)));

        assert_eq!(
            table.raw_next(Value::nil()),
            Some((Value::integer(1), Value::integer(10)))
        );
        assert_eq!(
            table.raw_next(Value::integer(1)),
            Some((Value::integer(3), Value::integer(30)))
        );
        assert_eq!(table.raw_next(Value::integer(3)), None);
    }

    #[test]
    fn table_raw_next_traverses_hash_entries() {
        let mut table = Table::new();
        assert!(table.raw_set_value(Value::boolean(true), Value::integer(1)));

        assert_eq!(
            table.raw_next(Value::nil()),
            Some((Value::boolean(true), Value::integer(1)))
        );
        assert_eq!(table.raw_next(Value::boolean(true)), None);
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

    #[test]
    fn table_meta_new_table_has_empty_metadata() {
        let table = Table::new();

        assert!(!table.has_metatable());
        assert_eq!(table.metatable(), None);
        assert_eq!(table.flags().bits(), 0);
        assert!(table.flags().is_empty());
        assert_eq!(table.version(), 0);
    }

    #[test]
    fn table_meta_set_metatable_updates_pointer_and_version() {
        let mut arena = GcArena::new();
        let metatable = arena.allocate(Table::new());
        let mut table = Table::new();

        table.set_metatable(Some(metatable));

        assert!(table.has_metatable());
        assert_eq!(table.metatable(), Some(metatable));
        assert!(table.flags().is_empty());
        assert_eq!(table.version(), 1);
    }

    #[test]
    fn table_meta_setting_same_metatable_does_not_bump_version() {
        let mut arena = GcArena::new();
        let metatable = arena.allocate(Table::new());
        let mut table = Table::new();

        table.set_metatable(Some(metatable));
        let version = table.version();
        table.set_metatable(Some(metatable));

        assert_eq!(table.metatable(), Some(metatable));
        assert_eq!(table.version(), version);
    }

    #[test]
    fn table_meta_clearing_metatable_bumps_version_once() {
        let mut arena = GcArena::new();
        let metatable = arena.allocate(Table::new());
        let mut table = Table::new();

        table.set_metatable(Some(metatable));
        table.set_metatable(None);
        let version = table.version();
        table.set_metatable(None);

        assert!(!table.has_metatable());
        assert_eq!(table.metatable(), None);
        assert_eq!(table.version(), version);
        assert_eq!(version, 2);
    }

    #[test]
    fn table_meta_array_version_changes_only_for_mutations() {
        let mut table = Table::new();

        assert!(table.raw_set_integer(1, Value::integer(7)));
        assert_eq!(table.version(), 1);

        assert!(table.raw_set_integer(1, Value::integer(7)));
        assert_eq!(table.version(), 1);

        assert!(table.raw_set_integer(3, Value::integer(9)));
        assert_eq!(table.version(), 2);

        assert!(table.raw_set_integer(2, Value::nil()));
        assert_eq!(table.version(), 2);

        assert!(table.raw_set_integer(3, Value::nil()));
        assert_eq!(table.version(), 3);
        assert_eq!(table.array_len(), 1);
    }

    #[test]
    fn table_meta_hash_version_changes_only_for_mutations() {
        let mut table = Table::new();

        assert!(table.raw_set_value(Value::boolean(true), Value::integer(1)));
        assert_eq!(table.version(), 1);

        assert!(table.raw_set_value(Value::boolean(true), Value::integer(1)));
        assert_eq!(table.version(), 1);

        assert!(table.raw_set_value(Value::boolean(true), Value::integer(2)));
        assert_eq!(table.version(), 2);

        assert!(table.raw_set_value(Value::boolean(true), Value::nil()));
        assert_eq!(table.version(), 3);

        assert!(table.raw_set_value(Value::boolean(true), Value::nil()));
        assert_eq!(table.version(), 3);
    }
}
