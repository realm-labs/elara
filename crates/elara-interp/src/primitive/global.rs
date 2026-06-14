//! Runtime global environment helpers.

use elara_bytecode::Instr;
use elara_core::{LuaThread, SHORT_STRING_MAX_BYTES, Value};

use super::{RuntimeErrorKind, RuntimeResult, RuntimeStrings, RuntimeTables, set_register};

/// Runtime-owned global environment for primitive execution.
#[derive(Default)]
pub(crate) struct RuntimeGlobals {
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

    fn get(&self, key: Value, tables: &mut RuntimeTables) -> RuntimeResult<Value> {
        tables.raw_get_cached(self.table_index as usize, key)
    }

    fn set(&mut self, key: Value, value: Value, tables: &mut RuntimeTables) -> RuntimeResult<()> {
        let table = tables
            .get_mut(self.table_index as usize)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        if table.raw_set_value(key, value) {
            Ok(())
        } else {
            Err(RuntimeErrorKind::InvalidTableKey.into())
        }
    }

    pub(super) fn get_named(
        &self,
        name: &[u8],
        strings: &mut RuntimeStrings,
        tables: &mut RuntimeTables,
    ) -> RuntimeResult<Value> {
        let key = global_key(name, strings)?;
        self.get(key, tables)
    }

    pub(super) fn set_named(
        &mut self,
        name: &[u8],
        value: Value,
        strings: &mut RuntimeStrings,
        tables: &mut RuntimeTables,
    ) -> RuntimeResult<()> {
        let key = global_key(name, strings)?;
        self.set(key, value, tables)
    }
}

pub(super) fn execute_get_env(
    thread: &mut LuaThread,
    instr: Instr,
    name: &[u8],
    globals: &RuntimeGlobals,
    strings: &mut RuntimeStrings,
    tables: &mut RuntimeTables,
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
        Err(RuntimeErrorKind::GlobalAlreadyDefined.into())
    }
}

fn global_key(name: &[u8], strings: &mut RuntimeStrings) -> RuntimeResult<Value> {
    if name.len() > SHORT_STRING_MAX_BYTES {
        return Err(RuntimeErrorKind::GlobalNameTooLong.into());
    }
    Ok(strings.intern_short_value(name))
}
