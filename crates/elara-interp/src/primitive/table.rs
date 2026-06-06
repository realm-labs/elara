//! Runtime table storage and table opcode helpers.

use std::ops::Index;

use elara_bytecode::Instr;
use elara_core::{LuaInteger, LuaThread, Table, Value};

use super::{RuntimeError, RuntimeResult, register, set_register};

/// Runtime-owned table storage for primitive execution.
#[derive(Default)]
pub struct RuntimeTables {
    tables: Vec<Table>,
    metatables: Vec<Option<u32>>,
}

impl RuntimeTables {
    /// Creates empty runtime table storage.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tables: Vec::new(),
            metatables: Vec::new(),
        }
    }

    /// Number of runtime-owned tables.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    /// Returns true when no tables are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Gets a runtime-owned table by placeholder index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Table> {
        self.tables.get(index)
    }

    /// Gets a mutable runtime-owned table by placeholder index.
    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Table> {
        self.tables.get_mut(index)
    }

    /// Gets a runtime table's metatable placeholder index.
    #[must_use]
    pub fn metatable(&self, index: usize) -> Option<u32> {
        self.metatables.get(index).copied().flatten()
    }

    /// Sets a runtime table's metatable placeholder index.
    pub fn set_metatable(&mut self, index: usize, metatable: Option<u32>) -> RuntimeResult<()> {
        if index >= self.tables.len() {
            return Err(RuntimeError::NonTableValue);
        }
        if let Some(metatable) = metatable
            && metatable as usize >= self.tables.len()
        {
            return Err(RuntimeError::NonTableValue);
        }
        self.metatables[index] = metatable;
        Ok(())
    }

    fn push(&mut self, table: Table) -> u32 {
        let table_index = u32::try_from(self.tables.len()).expect("runtime table index must fit");
        self.tables.push(table);
        self.metatables.push(None);
        table_index
    }
}

impl Index<usize> for RuntimeTables {
    type Output = Table;

    fn index(&self, index: usize) -> &Self::Output {
        &self.tables[index]
    }
}

pub(super) fn execute_vararg_table(
    thread: &mut LuaThread,
    instr: Instr,
    varargs: &[Value],
    tables: &mut RuntimeTables,
) -> RuntimeResult<()> {
    let mut table = Table::new();
    for (index, value) in varargs.iter().copied().enumerate() {
        let key =
            LuaInteger::try_from(index + 1).expect("vararg table index must fit in LuaInteger");
        table.raw_set_integer(key, value);
    }

    let table_index = tables.push(table);
    set_register(thread, instr.a().into(), Value::table_index(table_index))
}

pub(super) fn execute_new_table(
    thread: &mut LuaThread,
    instr: Instr,
    tables: &mut RuntimeTables,
) -> RuntimeResult<()> {
    let table_index = tables.push(Table::new());
    set_register(thread, instr.a().into(), Value::table_index(table_index))
}

pub(super) fn execute_set_table(
    thread: &mut LuaThread,
    instr: Instr,
    tables: &mut RuntimeTables,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.a().into())?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let key = register(thread, instr.b() as usize)?;
    let value = register(thread, instr.c() as usize)?;
    let table = tables
        .get_mut(table_index)
        .ok_or(RuntimeError::NonTableValue)?;
    if table.raw_set_value(key, value) {
        Ok(())
    } else {
        Err(RuntimeError::InvalidTableKey)
    }
}

pub(super) fn execute_get_table(
    thread: &mut LuaThread,
    instr: Instr,
    tables: &RuntimeTables,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.b() as usize)?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let key = register(thread, instr.c() as usize)?;
    let table = tables.get(table_index).ok_or(RuntimeError::NonTableValue)?;
    let value = table.raw_get_value(key);
    set_register(thread, instr.a().into(), value)
}

pub(super) fn execute_get_index(
    thread: &mut LuaThread,
    instr: Instr,
    tables: &RuntimeTables,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.b() as usize)?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let table = tables.get(table_index).ok_or(RuntimeError::NonTableValue)?;
    let value = table.raw_get_integer(LuaInteger::from(instr.c()));
    set_register(thread, instr.a().into(), value)
}

pub(super) fn execute_set_index(
    thread: &mut LuaThread,
    instr: Instr,
    tables: &mut RuntimeTables,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.a().into())?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let value = register(thread, instr.c() as usize)?;
    let table = tables
        .get_mut(table_index)
        .ok_or(RuntimeError::NonTableValue)?;
    if table.raw_set_integer(LuaInteger::from(instr.b()), value) {
        Ok(())
    } else {
        Err(RuntimeError::InvalidTableKey)
    }
}

#[cfg(test)]
mod tests {
    use elara_core::Table;

    use super::RuntimeTables;
    use crate::RuntimeError;

    #[test]
    fn metamethods_runtime_tables_store_metatable_links() {
        let mut tables = RuntimeTables::new();
        let table = tables.push(Table::new());
        let metatable = tables.push(Table::new());

        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        assert_eq!(tables.metatable(table as usize), Some(metatable));
    }

    #[test]
    fn metamethods_runtime_tables_reject_missing_metatable_links() {
        let mut tables = RuntimeTables::new();
        let table = tables.push(Table::new());

        assert_eq!(
            tables.set_metatable(table as usize, Some(99)),
            Err(RuntimeError::NonTableValue)
        );
        assert_eq!(tables.metatable(table as usize), None);
    }
}
