//! Runtime global environment helpers.

use elara_bytecode::Instr;
use elara_core::{LuaThread, SHORT_STRING_MAX_BYTES, Value};

use super::{RuntimeError, RuntimeResult, RuntimeStrings, RuntimeTables, set_register};

/// Runtime-owned global environment for primitive execution.
#[derive(Default)]
pub(super) struct RuntimeGlobals {
    table_index: u32,
}

impl RuntimeGlobals {
    /// Creates a global environment backed by a runtime-owned table.
    #[must_use]
    pub const fn new(table_index: u32) -> Self {
        Self { table_index }
    }

    /// Returns the Lua value for this global environment table.
    #[must_use]
    pub const fn value(&self) -> Value {
        Value::table_index(self.table_index)
    }

    fn get(&self, key: Value, tables: &RuntimeTables) -> RuntimeResult<Value> {
        let table = tables
            .get(self.table_index as usize)
            .ok_or(RuntimeError::NonTableValue)?;
        Ok(table.raw_get_value(key))
    }

    fn set(&mut self, key: Value, value: Value, tables: &mut RuntimeTables) -> RuntimeResult<()> {
        let table = tables
            .get_mut(self.table_index as usize)
            .ok_or(RuntimeError::NonTableValue)?;
        if table.raw_set_value(key, value) {
            Ok(())
        } else {
            Err(RuntimeError::InvalidTableKey)
        }
    }
}

pub(super) fn execute_get_env(
    thread: &mut LuaThread,
    instr: Instr,
    name: &[u8],
    globals: &RuntimeGlobals,
    strings: &mut RuntimeStrings,
    tables: &RuntimeTables,
) -> RuntimeResult<()> {
    let key = global_key(name, strings)?;
    set_register(thread, instr.a().into(), globals.get(key, tables)?)
}

pub(super) fn execute_set_env(
    thread: &LuaThread,
    instr: Instr,
    name: &[u8],
    globals: &mut RuntimeGlobals,
    strings: &mut RuntimeStrings,
    tables: &mut RuntimeTables,
) -> RuntimeResult<()> {
    let key = global_key(name, strings)?;
    let value = super::register(thread, instr.a().into())?;
    globals.set(key, value, tables)
}

pub(super) fn execute_decl_global(
    thread: &mut LuaThread,
    instr: Instr,
    name: &[u8],
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let _ = global_key(name, strings)?;
    let current = super::register(thread, instr.a().into())?;
    if current.is_nil() {
        Ok(())
    } else {
        Err(RuntimeError::GlobalAlreadyDefined)
    }
}

fn global_key(name: &[u8], strings: &mut RuntimeStrings) -> RuntimeResult<Value> {
    if name.len() > SHORT_STRING_MAX_BYTES {
        return Err(RuntimeError::GlobalNameTooLong);
    }
    Ok(strings.intern_short_value(name))
}
