//! Runtime table storage and table opcode helpers.

use std::ops::Index;

use elara_bytecode::Instr;
use elara_core::{LuaInteger, LuaThread, Table, Value};

use super::{
    RuntimeClosure, RuntimeError, RuntimeResult, RuntimeStrings, call_closure, register,
    set_register,
};

const MAX_TAG_METHOD_CHAIN: usize = 2000;
const INDEX_METAMETHOD: &str = "__index";
const NEWINDEX_METAMETHOD: &str = "__newindex";

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

    fn raw_get(&self, table_index: usize, key: Value) -> RuntimeResult<Value> {
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeError::NonTableValue)?;
        Ok(table.raw_get_value(key))
    }

    fn raw_set(&mut self, table_index: usize, key: Value, value: Value) -> RuntimeResult<()> {
        let table = self
            .tables
            .get_mut(table_index)
            .ok_or(RuntimeError::NonTableValue)?;
        if table.raw_set_value(key, value) {
            Ok(())
        } else {
            Err(RuntimeError::InvalidTableKey)
        }
    }

    fn raw_get_integer(&self, table_index: usize, key: LuaInteger) -> RuntimeResult<Value> {
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeError::NonTableValue)?;
        Ok(table.raw_get_integer(key))
    }

    fn raw_set_integer(
        &mut self,
        table_index: usize,
        key: LuaInteger,
        value: Value,
    ) -> RuntimeResult<()> {
        let table = self
            .tables
            .get_mut(table_index)
            .ok_or(RuntimeError::NonTableValue)?;
        if table.raw_set_integer(key, value) {
            Ok(())
        } else {
            Err(RuntimeError::InvalidTableKey)
        }
    }

    fn get_with_index(
        &mut self,
        table_index: usize,
        key: Value,
        closures: &mut Vec<RuntimeClosure>,
        strings: &mut RuntimeStrings,
    ) -> RuntimeResult<Value> {
        let mut current = table_index;
        for _ in 0..MAX_TAG_METHOD_CHAIN {
            let value = self.raw_get(current, key)?;
            if !value.is_nil() {
                return Ok(value);
            }

            let Some(metamethod) = self.metamethod(current, INDEX_METAMETHOD, strings)? else {
                return Ok(Value::nil());
            };
            if let Some(next) = metamethod.as_table_index() {
                current = next as usize;
                continue;
            }
            if let Some(closure) = metamethod.as_closure_index() {
                let receiver = Value::table_index(
                    u32::try_from(current).expect("runtime table index must fit in u32"),
                );
                let returns =
                    call_closure(closures, closure as usize, &[receiver, key], self, strings)?;
                return Ok(returns.first().copied().unwrap_or_else(Value::nil));
            }
            return Err(RuntimeError::UnsupportedMetamethod {
                name: INDEX_METAMETHOD,
            });
        }

        Err(RuntimeError::MetamethodChainTooLong {
            name: INDEX_METAMETHOD,
        })
    }

    fn set_with_newindex(
        &mut self,
        table_index: usize,
        key: Value,
        value: Value,
        closures: &mut Vec<RuntimeClosure>,
        strings: &mut RuntimeStrings,
    ) -> RuntimeResult<()> {
        let mut current = table_index;
        for _ in 0..MAX_TAG_METHOD_CHAIN {
            if !self.raw_get(current, key)?.is_nil() {
                return self.raw_set(current, key, value);
            }

            let Some(metamethod) = self.metamethod(current, NEWINDEX_METAMETHOD, strings)? else {
                return self.raw_set(current, key, value);
            };
            if let Some(next) = metamethod.as_table_index() {
                current = next as usize;
                continue;
            }
            if let Some(closure) = metamethod.as_closure_index() {
                let receiver = Value::table_index(
                    u32::try_from(current).expect("runtime table index must fit in u32"),
                );
                call_closure(
                    closures,
                    closure as usize,
                    &[receiver, key, value],
                    self,
                    strings,
                )?;
                return Ok(());
            }
            return Err(RuntimeError::UnsupportedMetamethod {
                name: NEWINDEX_METAMETHOD,
            });
        }

        Err(RuntimeError::MetamethodChainTooLong {
            name: NEWINDEX_METAMETHOD,
        })
    }

    fn metamethod(
        &mut self,
        table_index: usize,
        name: &'static str,
        strings: &mut RuntimeStrings,
    ) -> RuntimeResult<Option<Value>> {
        let Some(metatable) = self.metatable(table_index) else {
            return Ok(None);
        };
        let key = strings.intern_short_value(name);
        let value = self.raw_get(metatable as usize, key)?;
        Ok((!value.is_nil()).then_some(value))
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
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.a().into())?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let key = register(thread, instr.b() as usize)?;
    let value = register(thread, instr.c() as usize)?;
    tables.set_with_newindex(table_index, key, value, closures, strings)
}

pub(super) fn execute_get_table(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.b() as usize)?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let key = register(thread, instr.c() as usize)?;
    let value = tables.get_with_index(table_index, key, closures, strings)?;
    set_register(thread, instr.a().into(), value)
}

pub(super) fn execute_get_index(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.b() as usize)?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let key = LuaInteger::from(instr.c());
    let value = tables.raw_get_integer(table_index, key)?;
    let value = if value.is_nil() {
        tables.get_with_index(table_index, Value::integer(key), closures, strings)?
    } else {
        value
    };
    set_register(thread, instr.a().into(), value)
}

pub(super) fn execute_set_index(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.a().into())?
        .as_table_index()
        .ok_or(RuntimeError::NonTableValue)? as usize;
    let value = register(thread, instr.c() as usize)?;
    let key = LuaInteger::from(instr.b());
    if !tables.raw_get_integer(table_index, key)?.is_nil() {
        tables.raw_set_integer(table_index, key, value)
    } else {
        tables.set_with_newindex(table_index, Value::integer(key), value, closures, strings)
    }
}

#[cfg(test)]
mod tests {
    use elara_bytecode::{Op, ProtoBuilder, UpvalueDesc};
    use elara_core::{Table, Value};

    use super::{INDEX_METAMETHOD, NEWINDEX_METAMETHOD, RuntimeTables};
    use crate::primitive::{RuntimeClosure, RuntimeError, RuntimeStrings};

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

    #[test]
    fn metamethods_index_reads_from_table_valued_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("missing");
        let index_key = strings.intern_short_value(INDEX_METAMETHOD);

        let mut tables = RuntimeTables::new();
        let table = tables.push(Table::new());
        let mut fallback = Table::new();
        assert!(fallback.raw_set_value(key, Value::integer(42)));
        let fallback = tables.push(fallback);
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(index_key, Value::table_index(fallback)));
        let metatable = tables.push(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut closures = Vec::new();
        assert_eq!(
            tables.get_with_index(table as usize, key, &mut closures, &mut strings),
            Ok(Value::integer(42))
        );
    }

    #[test]
    fn metamethods_newindex_writes_to_table_valued_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("missing");
        let newindex_key = strings.intern_short_value(NEWINDEX_METAMETHOD);

        let mut tables = RuntimeTables::new();
        let table = tables.push(Table::new());
        let sink = tables.push(Table::new());
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(newindex_key, Value::table_index(sink)));
        let metatable = tables.push(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut closures = Vec::new();
        tables
            .set_with_newindex(
                table as usize,
                key,
                Value::integer(42),
                &mut closures,
                &mut strings,
            )
            .expect("table-valued __newindex should write");

        assert_eq!(tables[table as usize].raw_get_value(key), Value::nil());
        assert_eq!(tables[sink as usize].raw_get_value(key), Value::integer(42));
    }

    #[test]
    fn metamethods_newindex_existing_key_bypasses_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("present");
        let newindex_key = strings.intern_short_value(NEWINDEX_METAMETHOD);

        let mut tables = RuntimeTables::new();
        let mut table_value = Table::new();
        assert!(table_value.raw_set_value(key, Value::integer(1)));
        let table = tables.push(table_value);
        let sink = tables.push(Table::new());
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(newindex_key, Value::table_index(sink)));
        let metatable = tables.push(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut closures = Vec::new();
        tables
            .set_with_newindex(
                table as usize,
                key,
                Value::integer(2),
                &mut closures,
                &mut strings,
            )
            .expect("existing key should write directly");

        assert_eq!(tables[table as usize].raw_get_value(key), Value::integer(2));
        assert_eq!(tables[sink as usize].raw_get_value(key), Value::nil());
    }

    #[test]
    fn metamethods_index_calls_function_valued_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("missing");
        let index_key = strings.intern_short_value(INDEX_METAMETHOD);

        let mut function = ProtoBuilder::new().with_signature(1, 0, false);
        let value = function.add_constant(Value::integer(42));
        function.emit_abx(Op::LoadK, 0, u64::from(value));
        function.emit_abc(Op::Return, 0, 1, 0);

        let mut closures = vec![RuntimeClosure {
            proto: function.finish(),
            upvalues: Vec::new(),
        }];
        let mut tables = RuntimeTables::new();
        let table = tables.push(Table::new());
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(index_key, Value::closure_index(0)));
        let metatable = tables.push(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        assert_eq!(
            tables.get_with_index(table as usize, key, &mut closures, &mut strings),
            Ok(Value::integer(42))
        );
    }

    #[test]
    fn metamethods_newindex_calls_function_valued_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("missing");
        let newindex_key = strings.intern_short_value(NEWINDEX_METAMETHOD);

        let mut tables = RuntimeTables::new();
        let table = tables.push(Table::new());
        let sink = tables.push(Table::new());

        let mut function = ProtoBuilder::new().with_signature(4, 0, true);
        function.add_upvalue(UpvalueDesc::new(Some("sink"), true, 0));
        function.emit_abc(Op::GetUpvalue, 0, 0, 0);
        function.emit_abc(Op::Vararg, 1, 3, 0);
        function.emit_abc(Op::SetTable, 0, 2, 3);
        function.emit_abc(Op::Return, 0, 0, 0);

        let mut closures = vec![RuntimeClosure {
            proto: function.finish(),
            upvalues: vec![Value::table_index(sink)],
        }];
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(newindex_key, Value::closure_index(0)));
        let metatable = tables.push(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        tables
            .set_with_newindex(
                table as usize,
                key,
                Value::integer(42),
                &mut closures,
                &mut strings,
            )
            .expect("function-valued __newindex should call");

        assert_eq!(tables[table as usize].raw_get_value(key), Value::nil());
        assert_eq!(tables[sink as usize].raw_get_value(key), Value::integer(42));
    }
}
