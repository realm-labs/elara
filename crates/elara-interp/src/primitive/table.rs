//! Runtime table storage and table opcode helpers.

use std::ops::Index;

use elara_bytecode::Instr;
use elara_core::{LuaInteger, LuaThread, Table, Value};

use super::{
    RuntimeClosure, RuntimeErrorKind, RuntimeGlobals, RuntimeNatives, RuntimeResult,
    RuntimeStrings, call_closure, register, set_register,
};

const MAX_TAG_METHOD_CHAIN: usize = 2000;
const INDEX_METAMETHOD: &str = "__index";
const NEWINDEX_METAMETHOD: &str = "__newindex";

/// Runtime-owned table storage for primitive execution.
#[derive(Default)]
pub struct RuntimeTables {
    tables: Vec<Table>,
    metatables: Vec<Option<u32>>,
    inline_caches: Box<TableInlineCaches>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InlineCacheStats {
    pub hits: usize,
    pub misses: usize,
}

#[derive(Default)]
struct TableInlineCaches {
    raw_get: Option<RawGetCache>,
    integer_get: Option<IntegerGetCache>,
    stats: InlineCacheStats,
}

#[derive(Clone, Copy)]
struct RawGetCache {
    table_index: usize,
    table_version: u32,
    key: Value,
    value: Value,
}

#[derive(Clone, Copy)]
struct IntegerGetCache {
    table_index: usize,
    table_version: u32,
    key: LuaInteger,
    value: Value,
}

impl RuntimeTables {
    /// Creates empty runtime table storage.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            metatables: Vec::new(),
            inline_caches: Box::new(TableInlineCaches {
                raw_get: None,
                integer_get: None,
                stats: InlineCacheStats { hits: 0, misses: 0 },
            }),
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

    /// Inline cache statistics.
    #[must_use]
    pub const fn inline_cache_stats(&self) -> InlineCacheStats {
        self.inline_caches.stats
    }

    /// Sets a runtime table's metatable placeholder index.
    pub fn set_metatable(&mut self, index: usize, metatable: Option<u32>) -> RuntimeResult<()> {
        if index >= self.tables.len() {
            return Err(RuntimeErrorKind::NonTableValue.into());
        }
        if let Some(metatable) = metatable
            && metatable as usize >= self.tables.len()
        {
            return Err(RuntimeErrorKind::NonTableValue.into());
        }
        if self.metatables[index] == metatable {
            return Ok(());
        }
        self.metatables[index] = metatable;
        self.tables[index].invalidate_runtime_caches();
        Ok(())
    }

    /// Adds a runtime-owned table and returns its placeholder index.
    pub fn push_table(&mut self, table: Table) -> u32 {
        let table_index = u32::try_from(self.tables.len()).expect("runtime table index must fit");
        self.tables.push(table);
        self.metatables.push(None);
        table_index
    }

    /// Gets a metamethod by name for a Lua value when this runtime can model it.
    pub(super) fn metamethod_for_value(
        &mut self,
        value: Value,
        name: &'static str,
        strings: &mut RuntimeStrings,
    ) -> RuntimeResult<Option<Value>> {
        let Some(table_index) = value.as_table_index() else {
            return Ok(None);
        };
        self.metamethod(table_index as usize, name, strings)
    }

    fn raw_get(&self, table_index: usize, key: Value) -> RuntimeResult<Value> {
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        Ok(table.raw_get_value(key))
    }

    pub(super) fn raw_get_cached(
        &mut self,
        table_index: usize,
        key: Value,
    ) -> RuntimeResult<Value> {
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        let table_version = table.version();
        if let Some(cache) = self.inline_caches.raw_get
            && cache.table_index == table_index
            && cache.table_version == table_version
            && cache.key == key
        {
            self.inline_caches.stats.hits += 1;
            return Ok(cache.value);
        }

        let value = table.raw_get_value(key);
        self.inline_caches.raw_get = Some(RawGetCache {
            table_index,
            table_version,
            key,
            value,
        });
        self.inline_caches.stats.misses += 1;
        Ok(value)
    }

    fn raw_set(&mut self, table_index: usize, key: Value, value: Value) -> RuntimeResult<()> {
        let table = self
            .tables
            .get_mut(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        if table.raw_set_value(key, value) {
            Ok(())
        } else {
            Err(RuntimeErrorKind::InvalidTableKey.into())
        }
    }

    fn raw_get_integer(&self, table_index: usize, key: LuaInteger) -> RuntimeResult<Value> {
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        Ok(table.raw_get_integer(key))
    }

    fn raw_get_integer_cached(
        &mut self,
        table_index: usize,
        key: LuaInteger,
    ) -> RuntimeResult<Value> {
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        let table_version = table.version();
        if let Some(cache) = self.inline_caches.integer_get
            && cache.table_index == table_index
            && cache.table_version == table_version
            && cache.key == key
        {
            self.inline_caches.stats.hits += 1;
            return Ok(cache.value);
        }

        let value = table.raw_get_integer(key);
        self.inline_caches.integer_get = Some(IntegerGetCache {
            table_index,
            table_version,
            key,
            value,
        });
        self.inline_caches.stats.misses += 1;
        Ok(value)
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
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        if table.raw_set_integer(key, value) {
            Ok(())
        } else {
            Err(RuntimeErrorKind::InvalidTableKey.into())
        }
    }

    fn get_with_index(
        &mut self,
        table_index: usize,
        key: Value,
        closures: &mut Vec<RuntimeClosure>,
        strings: &mut RuntimeStrings,
        natives: &RuntimeNatives,
        globals: &mut RuntimeGlobals,
    ) -> RuntimeResult<Value> {
        let mut current = table_index;
        for _ in 0..MAX_TAG_METHOD_CHAIN {
            let value = self.raw_get_cached(current, key)?;
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
                let returns = call_closure(
                    closures,
                    closure as usize,
                    &[receiver, key],
                    self,
                    strings,
                    natives,
                    globals,
                    None,
                )?;
                return Ok(returns.first().copied().unwrap_or_else(Value::nil));
            }
            return Err(RuntimeErrorKind::UnsupportedMetamethod {
                name: INDEX_METAMETHOD,
            }
            .into());
        }

        Err(RuntimeErrorKind::MetamethodChainTooLong {
            name: INDEX_METAMETHOD,
        }
        .into())
    }

    #[allow(clippy::too_many_arguments)]
    fn set_with_newindex(
        &mut self,
        table_index: usize,
        key: Value,
        value: Value,
        closures: &mut Vec<RuntimeClosure>,
        strings: &mut RuntimeStrings,
        natives: &RuntimeNatives,
        globals: &mut RuntimeGlobals,
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
                    natives,
                    globals,
                    None,
                )?;
                return Ok(());
            }
            return Err(RuntimeErrorKind::UnsupportedMetamethod {
                name: NEWINDEX_METAMETHOD,
            }
            .into());
        }

        Err(RuntimeErrorKind::MetamethodChainTooLong {
            name: NEWINDEX_METAMETHOD,
        }
        .into())
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
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let mut table = Table::new();
    let count = LuaInteger::try_from(varargs.len()).expect("vararg count must fit in LuaInteger");
    let n_key = strings.intern_short_value("n");
    if !table.raw_set_value(n_key, Value::integer(count)) {
        return Err(RuntimeErrorKind::InvalidTableKey.into());
    }

    for (index, value) in varargs.iter().copied().enumerate() {
        let key =
            LuaInteger::try_from(index + 1).expect("vararg table index must fit in LuaInteger");
        table.raw_set_integer(key, value);
    }

    let table_index = tables.push_table(table);
    set_register(thread, instr.a().into(), Value::table_index(table_index))
}

pub(super) fn execute_new_table(
    thread: &mut LuaThread,
    instr: Instr,
    tables: &mut RuntimeTables,
) -> RuntimeResult<()> {
    let table_index = tables.push_table(Table::new());
    set_register(thread, instr.a().into(), Value::table_index(table_index))
}

pub(super) fn execute_set_table(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.a().into())?
        .as_table_index()
        .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
    let key = register(thread, instr.b() as usize)?;
    let value = register(thread, instr.c() as usize)?;
    tables.set_with_newindex(table_index, key, value, closures, strings, natives, globals)
}

pub(super) fn execute_get_table(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.b() as usize)?
        .as_table_index()
        .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
    let key = register(thread, instr.c() as usize)?;
    let value = tables.get_with_index(table_index, key, closures, strings, natives, globals)?;
    set_register(thread, instr.a().into(), value)
}

pub(super) fn execute_get_index(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.b() as usize)?
        .as_table_index()
        .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
    let key = LuaInteger::from(instr.c());
    let value = tables.raw_get_integer_cached(table_index, key)?;
    let value = if value.is_nil() {
        tables.get_with_index(
            table_index,
            Value::integer(key),
            closures,
            strings,
            natives,
            globals,
        )?
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
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<()> {
    let table_index = register(thread, instr.a().into())?
        .as_table_index()
        .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
    let value = register(thread, instr.c() as usize)?;
    let key = LuaInteger::from(instr.b());
    if !tables.raw_get_integer(table_index, key)?.is_nil() {
        tables.raw_set_integer(table_index, key, value)
    } else {
        tables.set_with_newindex(
            table_index,
            Value::integer(key),
            value,
            closures,
            strings,
            natives,
            globals,
        )
    }
}

#[cfg(test)]
mod tests {
    use elara_bytecode::{Op, ProtoBuilder, UpvalueDesc};
    use elara_core::{Table, Value};

    use super::{INDEX_METAMETHOD, NEWINDEX_METAMETHOD, RuntimeTables};
    use crate::primitive::{
        RuntimeClosure, RuntimeErrorKind, RuntimeGlobals, RuntimeNatives, RuntimeStrings,
        RuntimeUpvalue,
    };

    fn runtime_globals(tables: &mut RuntimeTables) -> RuntimeGlobals {
        let global_table = tables.push_table(Table::new());
        RuntimeGlobals::new(global_table)
    }

    #[test]
    fn metamethods_runtime_tables_store_metatable_links() {
        let mut tables = RuntimeTables::new();
        let table = tables.push_table(Table::new());
        let metatable = tables.push_table(Table::new());

        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        assert_eq!(tables.metatable(table as usize), Some(metatable));
    }

    #[test]
    fn metamethods_runtime_tables_reject_missing_metatable_links() {
        let mut tables = RuntimeTables::new();
        let table = tables.push_table(Table::new());

        assert_eq!(
            tables.set_metatable(table as usize, Some(99)),
            Err(RuntimeErrorKind::NonTableValue.into())
        );
        assert_eq!(tables.metatable(table as usize), None);
    }

    #[test]
    fn inline_cache_records_raw_get_hits_and_misses() {
        let key = Value::boolean(true);
        let mut table = Table::new();
        assert!(table.raw_set_value(key, Value::integer(1)));
        let mut tables = RuntimeTables::new();
        let table = tables.push_table(table) as usize;

        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(1)));
        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(1)));

        let stats = tables.inline_cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn inline_cache_invalidates_on_table_version_change() {
        let key = Value::boolean(true);
        let mut table = Table::new();
        assert!(table.raw_set_value(key, Value::integer(1)));
        let mut tables = RuntimeTables::new();
        let table = tables.push_table(table) as usize;

        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(1)));
        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(1)));
        tables
            .raw_set(table, key, Value::integer(2))
            .expect("table write should succeed");
        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(2)));

        let stats = tables.inline_cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
    }

    #[test]
    fn inline_cache_records_integer_get_hits() {
        let mut table = Table::new();
        assert!(table.raw_set_integer(1, Value::integer(7)));
        let mut tables = RuntimeTables::new();
        let table = tables.push_table(table) as usize;

        assert_eq!(
            tables.raw_get_integer_cached(table, 1),
            Ok(Value::integer(7))
        );
        assert_eq!(
            tables.raw_get_integer_cached(table, 1),
            Ok(Value::integer(7))
        );

        let stats = tables.inline_cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn inline_cache_invalidates_on_runtime_metatable_change() {
        let key = Value::boolean(true);
        let mut table = Table::new();
        assert!(table.raw_set_value(key, Value::integer(1)));
        let mut tables = RuntimeTables::new();
        let table = tables.push_table(table) as usize;
        let metatable = tables.push_table(Table::new());

        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(1)));
        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(1)));
        tables
            .set_metatable(table, Some(metatable))
            .expect("metatable link should be valid");
        assert_eq!(tables.raw_get_cached(table, key), Ok(Value::integer(1)));

        let stats = tables.inline_cache_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
    }

    #[test]
    fn metamethods_index_reads_from_table_valued_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("missing");
        let index_key = strings.intern_short_value(INDEX_METAMETHOD);

        let mut tables = RuntimeTables::new();
        let table = tables.push_table(Table::new());
        let mut fallback = Table::new();
        assert!(fallback.raw_set_value(key, Value::integer(42)));
        let fallback = tables.push_table(fallback);
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(index_key, Value::table_index(fallback)));
        let metatable = tables.push_table(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut closures = Vec::new();
        let mut globals = runtime_globals(&mut tables);
        let natives = RuntimeNatives::new();
        assert_eq!(
            tables.get_with_index(
                table as usize,
                key,
                &mut closures,
                &mut strings,
                &natives,
                &mut globals,
            ),
            Ok(Value::integer(42))
        );
    }

    #[test]
    fn metamethods_newindex_writes_to_table_valued_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("missing");
        let newindex_key = strings.intern_short_value(NEWINDEX_METAMETHOD);

        let mut tables = RuntimeTables::new();
        let table = tables.push_table(Table::new());
        let sink = tables.push_table(Table::new());
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(newindex_key, Value::table_index(sink)));
        let metatable = tables.push_table(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut closures = Vec::new();
        let mut globals = runtime_globals(&mut tables);
        let natives = RuntimeNatives::new();
        tables
            .set_with_newindex(
                table as usize,
                key,
                Value::integer(42),
                &mut closures,
                &mut strings,
                &natives,
                &mut globals,
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
        let table = tables.push_table(table_value);
        let sink = tables.push_table(Table::new());
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(newindex_key, Value::table_index(sink)));
        let metatable = tables.push_table(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut closures = Vec::new();
        let mut globals = runtime_globals(&mut tables);
        let natives = RuntimeNatives::new();
        tables
            .set_with_newindex(
                table as usize,
                key,
                Value::integer(2),
                &mut closures,
                &mut strings,
                &natives,
                &mut globals,
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
        let table = tables.push_table(Table::new());
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(index_key, Value::closure_index(0)));
        let metatable = tables.push_table(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut globals = runtime_globals(&mut tables);
        let natives = RuntimeNatives::new();
        assert_eq!(
            tables.get_with_index(
                table as usize,
                key,
                &mut closures,
                &mut strings,
                &natives,
                &mut globals,
            ),
            Ok(Value::integer(42))
        );
    }

    #[test]
    fn metamethods_newindex_calls_function_valued_fallback() {
        let mut strings = RuntimeStrings::new();
        let key = strings.intern_short_value("missing");
        let newindex_key = strings.intern_short_value(NEWINDEX_METAMETHOD);

        let mut tables = RuntimeTables::new();
        let table = tables.push_table(Table::new());
        let sink = tables.push_table(Table::new());

        let mut function = ProtoBuilder::new().with_signature(4, 0, true);
        function.add_upvalue(UpvalueDesc::new(Some("sink"), true, 0));
        function.emit_abc(Op::GetUpvalue, 0, 0, 0);
        function.emit_abc(Op::Vararg, 1, 3, 0);
        function.emit_abc(Op::SetTable, 0, 2, 3);
        function.emit_abc(Op::Return, 0, 0, 0);

        let mut closures = vec![RuntimeClosure {
            proto: function.finish(),
            upvalues: vec![RuntimeUpvalue::new(Value::table_index(sink))],
        }];
        let mut metatable = Table::new();
        assert!(metatable.raw_set_value(newindex_key, Value::closure_index(0)));
        let metatable = tables.push_table(metatable);
        tables
            .set_metatable(table as usize, Some(metatable))
            .expect("metatable link should be valid");

        let mut globals = runtime_globals(&mut tables);
        let natives = RuntimeNatives::new();
        tables
            .set_with_newindex(
                table as usize,
                key,
                Value::integer(42),
                &mut closures,
                &mut strings,
                &natives,
                &mut globals,
            )
            .expect("function-valued __newindex should call");

        assert_eq!(tables[table as usize].raw_get_value(key), Value::nil());
        assert_eq!(tables[sink as usize].raw_get_value(key), Value::integer(42));
    }
}
