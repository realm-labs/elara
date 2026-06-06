//! Runtime global environment helpers.

use elara_bytecode::Instr;
use elara_core::{LuaThread, SHORT_STRING_MAX_BYTES, Table, Value};

use super::{RuntimeError, RuntimeResult, RuntimeStrings, set_register};

/// Runtime-owned global environment for primitive execution.
#[derive(Default)]
pub(super) struct RuntimeGlobals {
    table: Table,
}

impl RuntimeGlobals {
    /// Creates an empty global environment.
    #[must_use]
    pub fn new() -> Self {
        Self {
            table: Table::new(),
        }
    }

    fn get(&self, key: Value) -> Value {
        self.table.raw_get_value(key)
    }

    fn set(&mut self, key: Value, value: Value) -> RuntimeResult<()> {
        if self.table.raw_set_value(key, value) {
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
) -> RuntimeResult<()> {
    let key = global_key(name, strings)?;
    set_register(thread, instr.a().into(), globals.get(key))
}

pub(super) fn execute_set_env(
    thread: &LuaThread,
    instr: Instr,
    name: &[u8],
    globals: &mut RuntimeGlobals,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let key = global_key(name, strings)?;
    let value = super::register(thread, instr.a().into())?;
    globals.set(key, value)
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
