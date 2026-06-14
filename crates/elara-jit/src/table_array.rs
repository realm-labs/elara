//! JIT table array fast-path guards and slow-path fallback reasons.

use elara_core::{LuaInteger, Value};
use elara_interp::primitive::RuntimeTables;

/// Version guard for a table array fast path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TableArrayGuard {
    table_index: u32,
    version: u32,
    array_len: usize,
}

impl TableArrayGuard {
    /// Runtime table index protected by this guard.
    #[must_use]
    pub const fn table_index(self) -> u32 {
        self.table_index
    }

    /// Table version protected by this guard.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }

    /// Array length observed when this guard was created.
    #[must_use]
    pub const fn array_len(self) -> usize {
        self.array_len
    }
}

/// Fast-path result or reason to call a slow helper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableArrayFastResult<T> {
    /// Fast path completed.
    Fast(T),
    /// Fast path guard failed and the caller should use a slow helper.
    Slow(TableArraySlowPath),
}

impl<T> TableArrayFastResult<T> {
    /// Returns true when the fast path completed.
    #[must_use]
    pub const fn is_fast(&self) -> bool {
        matches!(self, Self::Fast(_))
    }

    /// Returns the slow-path reason, if any.
    #[must_use]
    pub const fn slow_path(&self) -> Option<TableArraySlowPath> {
        match self {
            Self::Fast(_) => None,
            Self::Slow(reason) => Some(*reason),
        }
    }
}

/// Reason a table array fast path must fall back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableArraySlowPath {
    /// The receiver is not a runtime table value.
    NonTable,
    /// The key is not an integer value.
    NonIntegerKey,
    /// The integer key is not a valid 1-based Lua array index.
    InvalidArrayIndex,
    /// The integer key is outside the guarded array length.
    OutOfArrayBounds,
    /// The table value no longer matches the guarded table.
    TableChanged,
    /// The table version no longer matches the guard.
    VersionChanged,
    /// Runtime table storage does not contain the guarded table.
    MissingTable,
    /// The raw set could not be represented by table storage.
    InvalidSet,
}

/// JIT table array fast-path operations.
pub struct TableArrayFastPath;

impl TableArrayFastPath {
    /// Builds a guard for a runtime table value.
    #[must_use]
    pub fn guard(tables: &RuntimeTables, table: Value) -> Option<TableArrayGuard> {
        let table_index = table.as_table_index()?;
        let table = tables.get(table_index as usize)?;
        Some(TableArrayGuard {
            table_index,
            version: table.version(),
            array_len: table.array_len(),
        })
    }

    /// Attempts a version-guarded fast array get.
    pub fn get(
        tables: &RuntimeTables,
        table: Value,
        key: Value,
        guard: TableArrayGuard,
    ) -> TableArrayFastResult<Value> {
        let Some((table_index, key)) = checked_inputs(table, key, guard) else {
            return input_slow_path(table, key, guard);
        };
        if usize::try_from(key - 1).map_or(true, |offset| offset >= guard.array_len) {
            return TableArrayFastResult::Slow(TableArraySlowPath::OutOfArrayBounds);
        }
        let Some(table) = tables.get(table_index as usize) else {
            return TableArrayFastResult::Slow(TableArraySlowPath::MissingTable);
        };
        if table.version() != guard.version {
            return TableArrayFastResult::Slow(TableArraySlowPath::VersionChanged);
        }
        TableArrayFastResult::Fast(table.raw_get_integer(key))
    }

    /// Attempts a version-guarded fast array set.
    pub fn set(
        tables: &mut RuntimeTables,
        table: Value,
        key: Value,
        value: Value,
        guard: TableArrayGuard,
    ) -> TableArrayFastResult<()> {
        let Some((table_index, key)) = checked_inputs(table, key, guard) else {
            return input_slow_path(table, key, guard);
        };
        if usize::try_from(key - 1).map_or(true, |offset| offset >= guard.array_len) {
            return TableArrayFastResult::Slow(TableArraySlowPath::OutOfArrayBounds);
        }
        let Some(table) = tables.get_mut(table_index as usize) else {
            return TableArrayFastResult::Slow(TableArraySlowPath::MissingTable);
        };
        if table.version() != guard.version {
            return TableArrayFastResult::Slow(TableArraySlowPath::VersionChanged);
        }
        if table.raw_set_integer(key, value) {
            TableArrayFastResult::Fast(())
        } else {
            TableArrayFastResult::Slow(TableArraySlowPath::InvalidSet)
        }
    }
}

fn checked_inputs(table: Value, key: Value, guard: TableArrayGuard) -> Option<(u32, LuaInteger)> {
    let table_index = table.as_table_index()?;
    if table_index != guard.table_index {
        return None;
    }
    let key = key.as_integer()?;
    if key < 1 {
        return None;
    }
    Some((table_index, key))
}

fn input_slow_path<T>(table: Value, key: Value, guard: TableArrayGuard) -> TableArrayFastResult<T> {
    let Some(table_index) = table.as_table_index() else {
        return TableArrayFastResult::Slow(TableArraySlowPath::NonTable);
    };
    if table_index != guard.table_index {
        return TableArrayFastResult::Slow(TableArraySlowPath::TableChanged);
    }
    let Some(key) = key.as_integer() else {
        return TableArrayFastResult::Slow(TableArraySlowPath::NonIntegerKey);
    };
    if key < 1 {
        return TableArrayFastResult::Slow(TableArraySlowPath::InvalidArrayIndex);
    }
    TableArrayFastResult::Slow(TableArraySlowPath::OutOfArrayBounds)
}

#[cfg(test)]
mod tests {
    use elara_core::{Table, Value};
    use elara_interp::primitive::RuntimeTables;

    use super::{TableArrayFastPath, TableArrayFastResult, TableArraySlowPath};

    #[test]
    fn table_array_fast_get_hits_with_matching_version_guard() {
        let (tables, table) = table_with_array_values();
        let guard = TableArrayFastPath::guard(&tables, Value::table_index(table))
            .expect("table guard should build");

        assert_eq!(guard.array_len(), 2);
        assert_eq!(
            TableArrayFastPath::get(&tables, Value::table_index(table), Value::integer(2), guard,),
            TableArrayFastResult::Fast(Value::integer(20))
        );
    }

    #[test]
    fn table_array_fast_set_updates_existing_slot_and_invalidates_version() {
        let (mut tables, table) = table_with_array_values();
        let guard = TableArrayFastPath::guard(&tables, Value::table_index(table))
            .expect("table guard should build");

        assert_eq!(
            TableArrayFastPath::set(
                &mut tables,
                Value::table_index(table),
                Value::integer(1),
                Value::integer(99),
                guard,
            ),
            TableArrayFastResult::Fast(())
        );
        assert_eq!(
            tables
                .get(table as usize)
                .expect("table should exist")
                .raw_get_integer(1),
            Value::integer(99)
        );
        assert_eq!(
            TableArrayFastPath::get(&tables, Value::table_index(table), Value::integer(1), guard,),
            TableArrayFastResult::Slow(TableArraySlowPath::VersionChanged)
        );
    }

    #[test]
    fn table_array_fast_path_falls_back_on_version_guard_miss() {
        let (mut tables, table) = table_with_array_values();
        let guard = TableArrayFastPath::guard(&tables, Value::table_index(table))
            .expect("table guard should build");
        tables
            .get_mut(table as usize)
            .expect("table should exist")
            .raw_set_integer(1, Value::integer(11));

        assert_eq!(
            TableArrayFastPath::get(&tables, Value::table_index(table), Value::integer(1), guard,),
            TableArrayFastResult::Slow(TableArraySlowPath::VersionChanged)
        );
    }

    #[test]
    fn table_array_fast_path_falls_back_for_bad_tags_and_bounds() {
        let (tables, table) = table_with_array_values();
        let guard = TableArrayFastPath::guard(&tables, Value::table_index(table))
            .expect("table guard should build");

        assert_eq!(
            TableArrayFastPath::get(&tables, Value::integer(1), Value::integer(1), guard),
            TableArrayFastResult::Slow(TableArraySlowPath::NonTable)
        );
        assert_eq!(
            TableArrayFastPath::get(&tables, Value::table_index(table), Value::float(1.0), guard,),
            TableArrayFastResult::Slow(TableArraySlowPath::NonIntegerKey)
        );
        assert_eq!(
            TableArrayFastPath::get(&tables, Value::table_index(table), Value::integer(3), guard,),
            TableArrayFastResult::Slow(TableArraySlowPath::OutOfArrayBounds)
        );
    }

    fn table_with_array_values() -> (RuntimeTables, u32) {
        let mut table = Table::new();
        assert!(table.raw_set_integer(1, Value::integer(10)));
        assert!(table.raw_set_integer(2, Value::integer(20)));
        let mut tables = RuntimeTables::new();
        let table = tables.push_table(table);
        (tables, table)
    }
}
