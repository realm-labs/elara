//! Primitive bytecode execution.

use std::sync::Arc;

use elara_bytecode::{Instr, Op, Proto, VerifyError, verify_proto};
use elara_core::{
    CallFrame, GcArena, LuaError, LuaFloat, LuaInteger, LuaThread, ResultCount,
    SHORT_STRING_MAX_BYTES, StringInterner, Table, ThreadStatus, TraceFrame, Value,
};

mod environment;
mod global;
mod loops;
mod metamethod;
mod table;

use environment::InitialValue;
pub use environment::RuntimeEnvironment;
use global::{RuntimeGlobals, execute_decl_global, execute_get_env, execute_set_env};
use loops::{
    execute_generic_for_call, execute_generic_for_loop, execute_numeric_for_loop,
    prepare_numeric_for,
};
use metamethod::{execute_arithmetic, execute_comparison, execute_concat, execute_len};
pub use table::RuntimeTables;
use table::{
    execute_get_index, execute_get_table, execute_new_table, execute_set_index, execute_set_table,
    execute_vararg_table,
};

/// Result of executing one prototype.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Structured primitive interpreter runtime error.
pub type RuntimeError = LuaError<RuntimeErrorKind>;

/// Values and temporary runtime-owned tables produced by primitive execution.
pub struct RuntimeOutput {
    /// Returned Lua values.
    pub values: Vec<Value>,
    /// Runtime table storage referenced by table placeholder values.
    pub tables: RuntimeTables,
    /// Runtime string storage referenced by string values.
    pub strings: RuntimeStrings,
}

/// Native function callable by primitive execution.
pub type NativeFunction = dyn for<'a> Fn(&mut NativeContext<'a>, &[Value]) -> RuntimeResult<Vec<Value>>
    + Send
    + Sync
    + 'static;

/// Runtime services available to native functions during a call.
pub struct NativeContext<'a> {
    tables: &'a mut RuntimeTables,
    strings: &'a mut RuntimeStrings,
}

impl<'a> NativeContext<'a> {
    /// Interns a short string in the current runtime and returns it as a Lua value.
    pub fn intern_short_string(&mut self, bytes: impl AsRef<[u8]>) -> RuntimeResult<Value> {
        let bytes = bytes.as_ref();
        if bytes.len() > SHORT_STRING_MAX_BYTES {
            return Err(RuntimeErrorKind::StringConcatTooLong.into());
        }
        Ok(self.strings.intern_short_value(bytes))
    }

    /// Returns bytes for a short string owned by this runtime.
    #[must_use]
    pub fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        self.strings.short_string_bytes(value)
    }

    /// Allocates a runtime-owned Lua table from raw key/value entries.
    pub fn create_table<I>(&mut self, entries: I) -> RuntimeResult<Value>
    where
        I: IntoIterator<Item = (Value, Value)>,
    {
        let mut table = Table::new();
        for (key, value) in entries {
            if !table.raw_set_value(key, value) {
                return Err(RuntimeErrorKind::InvalidTableKey.into());
            }
        }
        Ok(Value::table_index(self.tables.push_table(table)))
    }

    /// Returns the current raw array length of a runtime-owned table.
    pub fn table_array_len(&self, table: Value) -> RuntimeResult<LuaInteger> {
        let table_index = table
            .as_table_index()
            .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
        let len = self
            .tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?
            .array_len();
        Ok(LuaInteger::try_from(len).expect("table array length must fit in LuaInteger"))
    }

    /// Reads one raw integer-keyed value from a runtime-owned table.
    pub fn table_get_integer(&self, table: Value, index: LuaInteger) -> RuntimeResult<Value> {
        self.table_get(table, Value::integer(index))
    }

    /// Reads one raw value-keyed value from a runtime-owned table.
    pub fn table_get(&self, table: Value, key: Value) -> RuntimeResult<Value> {
        let table_index = table
            .as_table_index()
            .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        Ok(table.raw_get_value(key))
    }

    /// Returns the next raw key/value pair after a key in a runtime-owned table.
    pub fn table_next(&self, table: Value, key: Value) -> RuntimeResult<Option<(Value, Value)>> {
        let table_index = table
            .as_table_index()
            .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
        let table = self
            .tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        Ok(table.raw_next(key))
    }

    /// Writes one raw integer-keyed value into a runtime-owned table.
    pub fn table_set_integer(
        &mut self,
        table: Value,
        index: LuaInteger,
        value: Value,
    ) -> RuntimeResult<()> {
        self.table_set(table, Value::integer(index), value)
    }

    /// Writes one raw value-keyed value into a runtime-owned table.
    pub fn table_set(&mut self, table: Value, key: Value, value: Value) -> RuntimeResult<()> {
        let table_index = table
            .as_table_index()
            .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
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

    /// Returns a runtime-owned table's metatable, or nil when absent.
    pub fn table_metatable(&self, table: Value) -> RuntimeResult<Value> {
        let table_index = table
            .as_table_index()
            .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
        self.tables
            .get(table_index)
            .ok_or(RuntimeErrorKind::NonTableValue)?;
        Ok(self
            .tables
            .metatable(table_index)
            .map_or_else(Value::nil, Value::table_index))
    }

    /// Sets a runtime-owned table's metatable to nil or another table.
    pub fn table_set_metatable(&mut self, table: Value, metatable: Value) -> RuntimeResult<()> {
        let table_index = table
            .as_table_index()
            .ok_or(RuntimeErrorKind::NonTableValue)? as usize;
        let metatable = if metatable.is_nil() {
            None
        } else {
            Some(
                metatable
                    .as_table_index()
                    .ok_or(RuntimeErrorKind::NonTableValue)?,
            )
        };
        self.tables.set_metatable(table_index, metatable)
    }
}

/// Runtime-owned native function registry.
#[derive(Clone, Default)]
pub struct RuntimeNatives {
    functions: Vec<Arc<NativeFunction>>,
}

impl RuntimeNatives {
    /// Creates an empty native function registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    /// Registers one native function and returns its runtime index.
    pub fn push<F>(&mut self, function: F) -> u32
    where
        F: for<'a> Fn(&mut NativeContext<'a>, &[Value]) -> RuntimeResult<Vec<Value>>
            + Send
            + Sync
            + 'static,
    {
        let index =
            u32::try_from(self.functions.len()).expect("native function index must fit in u32");
        self.functions.push(Arc::new(function));
        index
    }

    /// Registers an arg-only native function and returns its runtime index.
    pub fn push_simple<F>(&mut self, function: F) -> u32
    where
        F: Fn(&[Value]) -> RuntimeResult<Vec<Value>> + Send + Sync + 'static,
    {
        self.push(move |_context, args| function(args))
    }

    fn get(&self, index: usize) -> Option<&NativeFunction> {
        self.functions.get(index).map(AsRef::as_ref)
    }
}

/// Result of executing a prototype behind a protected-call boundary.
pub enum ProtectedRuntimeOutput {
    /// Execution completed normally.
    Ok(RuntimeOutput),
    /// Execution raised a runtime error that was caught by the boundary.
    Err(RuntimeError),
}

/// Result of resuming a primitive coroutine.
#[derive(Clone, Debug, PartialEq)]
pub enum CoroutineResume {
    /// Coroutine returned normally and is now dead.
    Return(Vec<Value>),
    /// Coroutine yielded values and is now suspended.
    Yield(Vec<Value>),
    /// Resume failed before or during execution.
    Error(RuntimeError),
}

/// Resumable primitive bytecode coroutine.
pub struct PrimitiveCoroutine {
    thread: LuaThread,
    closures: Vec<RuntimeClosure>,
    tables: RuntimeTables,
    strings: RuntimeStrings,
    natives: RuntimeNatives,
    globals: RuntimeGlobals,
    frames: Vec<CoroutineFrame>,
    to_be_closed: Vec<usize>,
}

#[derive(Debug)]
pub(super) struct CoroutineFrame {
    proto: Proto,
    upvalues: Vec<Value>,
    varargs: Vec<Value>,
    pc: usize,
    dynamic_top: usize,
    call_slot: Option<CoroutineCallSlot>,
    yielded_base: Option<usize>,
    tbc_start: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CoroutineCallSlot {
    base: usize,
    result_count: u32,
}

/// Runtime-owned string storage for primitive execution output.
#[derive(Default)]
pub struct RuntimeStrings {
    arena: GcArena,
    interner: StringInterner,
}

impl PrimitiveCoroutine {
    /// Creates a coroutine from a verified prototype.
    pub fn new(proto: Proto) -> RuntimeResult<Self> {
        verify_proto(&proto)
            .map_err(RuntimeErrorKind::Verification)
            .map_err(RuntimeError::from)?;
        let mut thread = LuaThread::new();
        for _ in 0..proto.max_stack {
            thread.push_value(Value::nil());
        }

        let mut tables = RuntimeTables::new();
        let global_table = tables.push_table(Table::new());
        let globals = RuntimeGlobals::new(global_table);
        let upvalues = vec![globals.value()];
        let frame = CoroutineFrame::new(proto, upvalues, Vec::new(), None, 0);

        Ok(Self {
            thread,
            closures: Vec::new(),
            tables,
            strings: RuntimeStrings::new(),
            natives: RuntimeNatives::new(),
            globals,
            frames: vec![frame],
            to_be_closed: Vec::new(),
        })
    }

    /// Current coroutine status.
    #[must_use]
    pub const fn status(&self) -> ThreadStatus {
        self.thread.status()
    }

    /// Runtime table storage owned by this coroutine.
    #[must_use]
    pub const fn tables(&self) -> &RuntimeTables {
        &self.tables
    }

    /// Runtime string storage owned by this coroutine.
    #[must_use]
    pub const fn strings(&self) -> &RuntimeStrings {
        &self.strings
    }

    /// Resumes the coroutine with Lua values.
    pub fn resume(&mut self, args: &[Value]) -> CoroutineResume {
        if !self.thread.enter_running() {
            let kind = if self.thread.status() == ThreadStatus::Dead {
                RuntimeErrorKind::CoroutineDead
            } else {
                RuntimeErrorKind::CoroutineNotSuspended
            };
            return CoroutineResume::Error(kind.into());
        }

        if let Err(error) = self.accept_resume_args(args) {
            self.thread.set_status(ThreadStatus::Suspended);
            return CoroutineResume::Error(error);
        }

        match self.run_until_pause() {
            Ok(CoroutinePause::Return(values)) => {
                self.thread.finish();
                CoroutineResume::Return(values)
            }
            Ok(CoroutinePause::Yield(values)) => {
                let _ = self.thread.suspend();
                CoroutineResume::Yield(values)
            }
            Err(error) => {
                let error = self.close_for_error(error);
                self.thread.finish();
                CoroutineResume::Error(error)
            }
        }
    }

    fn accept_resume_args(&mut self, args: &[Value]) -> RuntimeResult<()> {
        let Some(frame) = self.frames.last_mut() else {
            return Ok(());
        };
        if let Some(base) = frame.yielded_base.take() {
            write_values(
                &mut self.thread,
                base,
                args,
                u32::try_from(args.len()).expect("resume result count must fit"),
            )?;
            frame.dynamic_top = base + args.len();
        } else {
            frame.varargs.clear();
            frame.varargs.extend_from_slice(args);
        }
        Ok(())
    }

    fn run_until_pause(&mut self) -> RuntimeResult<CoroutinePause> {
        loop {
            let Some(frame) = self.frames.last_mut() else {
                return Ok(CoroutinePause::Return(Vec::new()));
            };
            if frame.pc >= frame.proto.code.len() {
                let returns = Vec::new();
                if let Some(values) = self.finish_frame(returns)? {
                    return Ok(CoroutinePause::Return(values));
                }
                continue;
            }

            let instr = frame.proto.code[frame.pc];
            frame.pc += 1;
            let mut context = ExecutionContext {
                closures: &mut self.closures,
                tables: &mut self.tables,
                strings: &mut self.strings,
                natives: &self.natives,
                globals: &mut self.globals,
                to_be_closed: &mut self.to_be_closed,
            };

            match execute_instruction(
                &frame.proto,
                &mut self.thread,
                &frame.upvalues,
                &frame.varargs,
                &mut context,
                &mut frame.dynamic_top,
                &mut frame.pc,
                instr,
                ExecutionMode::Coroutine,
            )? {
                InstructionFlow::Continue => {}
                InstructionFlow::Return(values) => {
                    if let Some(values) = self.finish_frame(values)? {
                        return Ok(CoroutinePause::Return(values));
                    }
                    continue;
                }
                InstructionFlow::Call {
                    closure_index,
                    args,
                    base,
                    result_count,
                } => {
                    let frame =
                        self.coroutine_call_frame(closure_index, args, base, result_count)?;
                    self.frames.push(frame);
                }
                InstructionFlow::Yield { values, base } => {
                    if let Some(frame) = self.frames.last_mut() {
                        frame.yielded_base = Some(base);
                    }
                    return Ok(CoroutinePause::Yield(values));
                }
            }
        }
    }

    fn coroutine_call_frame(
        &self,
        closure_index: usize,
        args: Vec<Value>,
        base: usize,
        result_count: u32,
    ) -> RuntimeResult<CoroutineFrame> {
        let closure = self
            .closures
            .get(closure_index)
            .cloned()
            .ok_or(RuntimeErrorKind::NonCallableValue)?;
        Ok(CoroutineFrame::new(
            closure.proto,
            closure.upvalues,
            args,
            Some(CoroutineCallSlot { base, result_count }),
            self.to_be_closed.len(),
        ))
    }

    fn finish_frame(&mut self, values: Vec<Value>) -> RuntimeResult<Option<Vec<Value>>> {
        let Some(frame) = self.frames.pop() else {
            return Ok(Some(values));
        };
        close_to_count(
            &self.thread,
            &mut self.closures,
            &mut self.tables,
            &mut self.strings,
            &self.natives,
            &mut self.globals,
            &mut self.to_be_closed,
            frame.tbc_start,
        )?;
        let Some(slot) = frame.call_slot else {
            return Ok(Some(values));
        };
        let Some(parent) = self.frames.last_mut() else {
            return Ok(Some(values));
        };
        let top = write_values(&mut self.thread, slot.base, &values, slot.result_count)?;
        if slot.result_count == 0 {
            parent.dynamic_top = top;
        }
        Ok(None)
    }

    fn close_for_error(&mut self, error: RuntimeError) -> RuntimeError {
        match close_to_count(
            &self.thread,
            &mut self.closures,
            &mut self.tables,
            &mut self.strings,
            &self.natives,
            &mut self.globals,
            &mut self.to_be_closed,
            0,
        ) {
            Ok(()) => error,
            Err(close_error) => close_error,
        }
    }
}

impl CoroutineFrame {
    fn new(
        proto: Proto,
        upvalues: Vec<Value>,
        varargs: Vec<Value>,
        call_slot: Option<CoroutineCallSlot>,
        tbc_start: usize,
    ) -> Self {
        Self {
            proto,
            upvalues,
            varargs,
            pc: 0,
            dynamic_top: 0,
            call_slot,
            yielded_base: None,
            tbc_start,
        }
    }

    #[cfg(test)]
    pub(super) fn test_root(proto: Proto, tbc_start: usize) -> Self {
        Self::new(proto, Vec::new(), Vec::new(), None, tbc_start)
    }
}

impl RuntimeStrings {
    /// Creates empty runtime string storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a short string and returns it as a Lua value.
    pub fn intern_short_value(&mut self, bytes: impl AsRef<[u8]>) -> Value {
        Value::short_string(self.interner.intern_short(&mut self.arena, bytes))
    }

    fn short_string_bytes(&self, value: Value) -> Option<&[u8]> {
        let string = value.as_short_string()?;
        // SAFETY: Primitive execution only creates short-string values through
        // this `RuntimeStrings` arena/interner, and `self` owns that storage for
        // at least as long as returned runtime values can be inspected.
        Some(unsafe { string.as_ref() }.as_bytes())
    }
}

/// Primitive interpreter runtime error kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeErrorKind {
    /// Bytecode verifier rejected the prototype.
    Verification(Vec<VerifyError>),
    /// Instruction tried to read an invalid constant.
    ConstantOutOfBounds { index: usize },
    /// Instruction tried to access an invalid register.
    RegisterOutOfBounds { register: usize },
    /// Instruction tried to access an invalid string constant.
    StringOutOfBounds { index: usize },
    /// Arithmetic operand was not numeric.
    NonNumericOperand { op: Op },
    /// Comparison operand was not comparable.
    NonComparableOperand { op: Op },
    /// Length operator received a value without primitive length or `__len`.
    NonLengthOperand,
    /// Concatenation received values without primitive concat or `__concat`.
    NonConcatOperand,
    /// Short-string concatenation exceeded current runtime string storage.
    StringConcatTooLong,
    /// Global declaration initialization found an already-defined global.
    GlobalAlreadyDefined,
    /// Global name exceeded current runtime short-string storage.
    GlobalNameTooLong,
    /// Table operation received a non-table receiver.
    NonTableValue,
    /// Table write used an invalid Lua key.
    InvalidTableKey,
    /// Metamethod dispatch found a metamethod shape this interpreter cannot call yet.
    UnsupportedMetamethod { name: &'static str },
    /// Metamethod table chain exceeded Lua's loop limit.
    MetamethodChainTooLong { name: &'static str },
    /// Call operand was not callable.
    NonCallableValue,
    /// Closure referenced a missing child prototype.
    ChildOutOfBounds { index: usize },
    /// Upvalue read referenced a missing captured value.
    UpvalueOutOfBounds { index: usize },
    /// Jump instruction computed an invalid program counter.
    JumpOutOfBounds { target: isize },
    /// Numeric for-loop operand was not numeric.
    ForLoopNonNumeric { operand: &'static str },
    /// Numeric for-loop step was zero.
    ForLoopStepZero,
    /// Numeric for-loop iteration count exceeded the current runtime storage.
    ForLoopCountOverflow,
    /// Tried to yield outside a coroutine resume boundary.
    YieldOutsideCoroutine,
    /// Tried to resume a coroutine that is already running or otherwise unavailable.
    CoroutineNotSuspended,
    /// Tried to resume a dead coroutine.
    CoroutineDead,
    /// To-be-closed variable received a value without a close metamethod.
    NonClosableValue,
    /// To-be-closed variable has a close metamethod this interpreter cannot call yet.
    UnsupportedCloseMetamethod,
    /// Native function index was not registered in this runtime.
    NativeFunctionOutOfBounds { index: usize },
    /// Native function raised a host/runtime error.
    NativeFunctionError { message: Box<str> },
    /// Opcode is not supported by the primitive interpreter.
    UnsupportedOpcode { op: Op },
}

impl RuntimeErrorKind {
    fn message(&self) -> String {
        match self {
            Self::Verification(errors) => format!("bytecode verification failed: {errors:?}"),
            Self::ConstantOutOfBounds { index } => {
                format!("constant index {index} is out of bounds")
            }
            Self::RegisterOutOfBounds { register } => {
                format!("register {register} is out of bounds")
            }
            Self::StringOutOfBounds { index } => {
                format!("string constant index {index} is out of bounds")
            }
            Self::NonNumericOperand { op } => {
                format!(
                    "attempt to perform '{}' on a non-number value",
                    op.mnemonic()
                )
            }
            Self::NonComparableOperand { op } => {
                format!("attempt to compare values with '{}'", op.mnemonic())
            }
            Self::NonLengthOperand => "attempt to get length of a non-table value".to_owned(),
            Self::NonConcatOperand => "attempt to concatenate unsupported values".to_owned(),
            Self::StringConcatTooLong => "string concatenation result is too long".to_owned(),
            Self::GlobalAlreadyDefined => "global already defined".to_owned(),
            Self::GlobalNameTooLong => "global name is too long".to_owned(),
            Self::NonTableValue => "attempt to index a non-table value".to_owned(),
            Self::InvalidTableKey => "table index is nil or NaN".to_owned(),
            Self::UnsupportedMetamethod { name } => {
                format!("unsupported metamethod shape for '{name}'")
            }
            Self::MetamethodChainTooLong { name } => {
                format!("'{name}' chain too long; possible loop")
            }
            Self::NonCallableValue => "attempt to call a non-function value".to_owned(),
            Self::ChildOutOfBounds { index } => {
                format!("child prototype index {index} is out of bounds")
            }
            Self::UpvalueOutOfBounds { index } => {
                format!("upvalue index {index} is out of bounds")
            }
            Self::JumpOutOfBounds { target } => {
                format!("jump target {target} is out of bounds")
            }
            Self::ForLoopNonNumeric { operand } => {
                format!("bad 'for' {operand} (number expected)")
            }
            Self::ForLoopStepZero => "'for' step is zero".to_owned(),
            Self::ForLoopCountOverflow => "numeric for-loop count overflow".to_owned(),
            Self::YieldOutsideCoroutine => "attempt to yield from outside a coroutine".to_owned(),
            Self::CoroutineNotSuspended => "cannot resume non-suspended coroutine".to_owned(),
            Self::CoroutineDead => "cannot resume dead coroutine".to_owned(),
            Self::NonClosableValue => "to-be-closed variable got a non-closable value".to_owned(),
            Self::UnsupportedCloseMetamethod => "unsupported '__close' metamethod shape".to_owned(),
            Self::NativeFunctionOutOfBounds { index } => {
                format!("native function index {index} is out of bounds")
            }
            Self::NativeFunctionError { message } => message.to_string(),
            Self::UnsupportedOpcode { op } => format!("unsupported opcode '{}'", op.mnemonic()),
        }
    }
}

fn runtime_error(kind: RuntimeErrorKind) -> RuntimeError {
    let message = kind.message();
    RuntimeError::new(kind, message)
}

impl From<RuntimeErrorKind> for RuntimeError {
    fn from(kind: RuntimeErrorKind) -> Self {
        runtime_error(kind)
    }
}

/// Executes a verified prototype and returns the first return values.
pub fn execute_proto(proto: &Proto) -> RuntimeResult<Vec<Value>> {
    execute_proto_with_output(proto).map(|output| output.values)
}

/// Executes a verified prototype and returns values plus runtime-owned tables.
pub fn execute_proto_with_output(proto: &Proto) -> RuntimeResult<RuntimeOutput> {
    execute_proto_with_output_in_mode(proto, false, RuntimeEnvironment::new())
}

/// Executes a verified prototype with a native function registry.
pub fn execute_proto_with_natives(
    proto: &Proto,
    natives: RuntimeNatives,
) -> RuntimeResult<RuntimeOutput> {
    execute_proto_with_environment(proto, RuntimeEnvironment::with_natives(natives))
}

/// Executes a verified prototype with an initial runtime environment.
pub fn execute_proto_with_environment(
    proto: &Proto,
    environment: RuntimeEnvironment,
) -> RuntimeResult<RuntimeOutput> {
    execute_proto_with_output_in_mode(proto, false, environment)
}

/// Executes a prototype and catches runtime errors at a protected boundary.
pub fn execute_proto_protected(proto: &Proto) -> ProtectedRuntimeOutput {
    let result = execute_proto_with_output_in_mode(proto, true, RuntimeEnvironment::new());
    match result {
        Ok(output) => ProtectedRuntimeOutput::Ok(output),
        Err(error) => ProtectedRuntimeOutput::Err(error),
    }
}

fn execute_proto_with_output_in_mode(
    proto: &Proto,
    protected: bool,
    environment: RuntimeEnvironment,
) -> RuntimeResult<RuntimeOutput> {
    verify_proto(proto)
        .map_err(RuntimeErrorKind::Verification)
        .map_err(RuntimeError::from)?;
    let (natives, initial_globals) = environment.into_parts();
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let global_table = tables.push_table(Table::new());
    let mut globals = RuntimeGlobals::new(global_table);
    for initial_global in initial_globals {
        let value = seed_initial_value(initial_global.value(), &mut tables, &mut strings)?;
        globals.set_named(initial_global.name(), value, &mut strings, &mut tables)?;
    }
    let global_value = globals.value();
    let mut to_be_closed = Vec::new();
    let mut context = ExecutionContext {
        closures: &mut closures,
        tables: &mut tables,
        strings: &mut strings,
        natives: &natives,
        globals: &mut globals,
        to_be_closed: &mut to_be_closed,
    };
    let values = execute_proto_with_upvalues(proto, &[global_value], &[], &mut context, protected)?;
    Ok(RuntimeOutput {
        values,
        tables,
        strings,
    })
}

fn seed_initial_value(
    value: &InitialValue,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Value> {
    match value {
        InitialValue::Value(value) => Ok(*value),
        InitialValue::Table(fields) => {
            let mut table = Table::new();
            for field in fields {
                let key = environment_key(field.name(), strings)?;
                if !table.raw_set_value(key, field.value()) {
                    return Err(RuntimeErrorKind::InvalidTableKey.into());
                }
            }
            Ok(Value::table_index(tables.push_table(table)))
        }
    }
}

fn environment_key(name: &[u8], strings: &mut RuntimeStrings) -> RuntimeResult<Value> {
    if name.len() > SHORT_STRING_MAX_BYTES {
        return Err(RuntimeErrorKind::GlobalNameTooLong.into());
    }
    Ok(strings.intern_short_value(name))
}

fn execute_proto_with_upvalues(
    proto: &Proto,
    upvalues: &[Value],
    varargs: &[Value],
    context: &mut ExecutionContext<'_>,
    protected: bool,
) -> RuntimeResult<Vec<Value>> {
    let mut thread = LuaThread::new();
    for _ in 0..proto.max_stack {
        thread.push_value(Value::nil());
    }
    if protected {
        let _ = thread.push_frame(
            CallFrame::new(0, usize::from(proto.max_stack), ResultCount::Multiple).protected(),
        );
    }
    let mut dynamic_top = 0;
    let mut pc = 0;
    let result = loop {
        if pc >= proto.code.len() {
            break Ok(Vec::new());
        }
        let instr = proto.code[pc];
        pc += 1;
        let flow = execute_instruction(
            proto,
            &mut thread,
            upvalues,
            varargs,
            context,
            &mut dynamic_top,
            &mut pc,
            instr,
            ExecutionMode::OneShot,
        );
        match flow {
            Err(error) => break Err(error),
            Ok(InstructionFlow::Continue) => {}
            Ok(InstructionFlow::Return(values)) => break Ok(values),
            Ok(InstructionFlow::Call { .. }) => {
                unreachable!("one-shot execution handles calls inline")
            }
            Ok(InstructionFlow::Yield { .. }) => {
                break Err(RuntimeErrorKind::YieldOutsideCoroutine.into());
            }
        }
    };

    match result {
        Ok(values) => Ok(values),
        Err(error) => {
            close_to_count(
                &thread,
                context.closures,
                context.tables,
                context.strings,
                context.natives,
                context.globals,
                context.to_be_closed,
                0,
            )?;
            Err(error)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_instruction(
    proto: &Proto,
    thread: &mut LuaThread,
    upvalues: &[Value],
    varargs: &[Value],
    context: &mut ExecutionContext<'_>,
    dynamic_top: &mut usize,
    pc: &mut usize,
    instr: Instr,
    mode: ExecutionMode,
) -> RuntimeResult<InstructionFlow> {
    match instr.op() {
        Op::Move => {
            let value = register(thread, instr.b() as usize)?;
            set_register(thread, instr.a().into(), value)?;
        }
        Op::LoadNil => set_register(thread, instr.a().into(), Value::nil())?,
        Op::LoadBool => set_register(thread, instr.a().into(), Value::boolean(instr.b() != 0))?,
        Op::LoadInt => set_register(
            thread,
            instr.a().into(),
            Value::integer(LuaInteger::from(instr.b())),
        )?,
        Op::LoadFloat => set_register(
            thread,
            instr.a().into(),
            Value::float(LuaFloat::from(instr.b())),
        )?,
        Op::LoadK => {
            let constant = proto.constants.get(instr.bx() as usize).copied().ok_or(
                RuntimeErrorKind::ConstantOutOfBounds {
                    index: instr.bx() as usize,
                },
            )?;
            set_register(thread, instr.a().into(), constant)?;
        }
        Op::LoadString => {
            let string = proto.string_constants.get(instr.bx() as usize).ok_or(
                RuntimeErrorKind::StringOutOfBounds {
                    index: instr.bx() as usize,
                },
            )?;
            let value = context.strings.intern_short_value(string);
            set_register(thread, instr.a().into(), value)?;
        }
        Op::GetEnv => {
            let name = string_constant(proto, instr)?;
            execute_get_env(
                thread,
                instr,
                name,
                context.globals,
                context.strings,
                context.tables,
            )?;
        }
        Op::SetEnv => {
            let name = string_constant(proto, instr)?;
            execute_set_env(
                thread,
                instr,
                name,
                context.globals,
                context.strings,
                context.tables,
            )?;
        }
        Op::DeclGlobal => {
            let name = string_constant(proto, instr)?;
            execute_decl_global(thread, instr, name, context.strings)?;
        }
        Op::NewTable => execute_new_table(thread, instr, context.tables)?,
        Op::GetTable => execute_get_table(
            thread,
            context.closures,
            instr,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
        )?,
        Op::SetTable => execute_set_table(
            thread,
            context.closures,
            instr,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
        )?,
        Op::GetIndex => execute_get_index(
            thread,
            context.closures,
            instr,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
        )?,
        Op::SetIndex => execute_set_index(
            thread,
            context.closures,
            instr,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
        )?,
        Op::GetUpvalue => {
            let value = upvalues.get(instr.b() as usize).copied().ok_or(
                RuntimeErrorKind::UpvalueOutOfBounds {
                    index: instr.b() as usize,
                },
            )?;
            set_register(thread, instr.a().into(), value)?;
        }
        Op::Closure => {
            let child_index = instr.bx() as usize;
            let child = proto
                .children
                .get(child_index)
                .cloned()
                .ok_or(RuntimeErrorKind::ChildOutOfBounds { index: child_index })?;
            let closure_index = context.closures.len();
            context.closures.push(RuntimeClosure {
                proto: child.clone(),
                upvalues: Vec::new(),
            });
            set_register(
                thread,
                instr.a().into(),
                Value::closure_index(closure_index as u32),
            )?;
            let captured = capture_upvalues(&child, thread, upvalues)?;
            context.closures[closure_index].upvalues = captured;
        }
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::IDiv | Op::Mod | Op::Pow | Op::Unm => {
            execute_arithmetic(
                thread,
                context.closures,
                instr,
                context.tables,
                context.strings,
                context.natives,
                context.globals,
            )?
        }
        Op::Len => execute_len(
            thread,
            context.closures,
            instr,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
        )?,
        Op::Concat => execute_concat(
            thread,
            context.closures,
            instr,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
        )?,
        Op::Eq | Op::Lt | Op::Le => {
            execute_comparison(
                thread,
                context.closures,
                instr,
                context.tables,
                context.strings,
                context.natives,
                context.globals,
            )?;
        }
        Op::Jmp => *pc = jump_target(*pc, instr)?,
        Op::ForPrep => {
            if prepare_numeric_for(thread, instr)? {
                *pc = jump_target(*pc, instr)?;
            }
        }
        Op::ForLoop => {
            if execute_numeric_for_loop(thread, instr)? {
                *pc = jump_target(*pc, instr)?;
            }
        }
        Op::TForPrep => *pc = jump_target(*pc, instr)?,
        Op::TForCall => {
            execute_generic_for_call(
                thread,
                context.closures,
                instr,
                context.tables,
                context.strings,
                context.natives,
                context.globals,
            )?;
        }
        Op::TForLoop => {
            if execute_generic_for_loop(thread, instr)? {
                *pc = jump_target(*pc, instr)?;
            }
        }
        Op::Test => {
            let value = register(thread, instr.a().into())?;
            if is_truthy(value) != (instr.b() != 0) {
                *pc += 1;
            }
        }
        Op::Vararg => {
            if let Some(top) = execute_vararg(thread, instr, varargs)? {
                *dynamic_top = top;
            }
        }
        Op::VarargTable => execute_vararg_table(thread, instr, varargs, context.tables)?,
        Op::Call => {
            let callee = callable_function(thread, context.tables, context.strings, instr)?;
            match callee {
                CallableFunction::Lua {
                    closure_index,
                    args,
                } if mode == ExecutionMode::Coroutine => {
                    return Ok(InstructionFlow::Call {
                        closure_index,
                        args,
                        base: usize::from(instr.a()),
                        result_count: instr.c(),
                    });
                }
                callable => {
                    let returns = call_function(callable, context)?;
                    if let Some(top) = write_call_returns(thread, instr, &returns)? {
                        *dynamic_top = top;
                    }
                }
            }
        }
        Op::Tbc => execute_tbc(thread, context, instr)?,
        Op::Close => close_to_base(
            thread,
            context.closures,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
            context.to_be_closed,
            usize::from(instr.a()),
        )?,
        Op::Return => {
            close_to_count(
                thread,
                context.closures,
                context.tables,
                context.strings,
                context.natives,
                context.globals,
                context.to_be_closed,
                0,
            )?;
            return collect_returns(thread, instr, *dynamic_top).map(InstructionFlow::Return);
        }
        Op::Yield => {
            let values = collect_returns(thread, instr, *dynamic_top)?;
            return Ok(InstructionFlow::Yield {
                values,
                base: usize::from(instr.a()),
            });
        }
        op => return Err(RuntimeErrorKind::UnsupportedOpcode { op }.into()),
    }

    Ok(InstructionFlow::Continue)
}

enum InstructionFlow {
    Continue,
    Return(Vec<Value>),
    Call {
        closure_index: usize,
        args: Vec<Value>,
        base: usize,
        result_count: u32,
    },
    Yield {
        values: Vec<Value>,
        base: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExecutionMode {
    OneShot,
    Coroutine,
}

enum CallableFunction {
    Lua {
        closure_index: usize,
        args: Vec<Value>,
    },
    Native {
        native_index: usize,
        args: Vec<Value>,
    },
}

enum CoroutinePause {
    Return(Vec<Value>),
    Yield(Vec<Value>),
}

fn jump_target(pc: usize, instr: Instr) -> RuntimeResult<usize> {
    let target = pc as isize + instr.sbx() as isize;
    usize::try_from(target)
        .map_err(|_| RuntimeErrorKind::JumpOutOfBounds { target })
        .map_err(RuntimeError::from)
}

fn string_constant(proto: &Proto, instr: Instr) -> RuntimeResult<&[u8]> {
    proto
        .string_constants
        .get(instr.bx() as usize)
        .map(Box::as_ref)
        .ok_or_else(|| RuntimeErrorKind::StringOutOfBounds {
            index: instr.bx() as usize,
        })
        .map_err(RuntimeError::from)
}

fn is_truthy(value: Value) -> bool {
    !value.is_nil() && value.as_bool() != Some(false)
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimeClosure {
    proto: Proto,
    upvalues: Vec<Value>,
}

pub(super) struct ExecutionContext<'a> {
    closures: &'a mut Vec<RuntimeClosure>,
    tables: &'a mut RuntimeTables,
    strings: &'a mut RuntimeStrings,
    natives: &'a RuntimeNatives,
    globals: &'a mut RuntimeGlobals,
    to_be_closed: &'a mut Vec<usize>,
}

fn capture_upvalues(
    child: &Proto,
    thread: &LuaThread,
    parent_upvalues: &[Value],
) -> RuntimeResult<Vec<Value>> {
    let mut captured = Vec::with_capacity(child.upvalues.len());
    for upvalue in &child.upvalues {
        let value = if upvalue.in_stack {
            register(thread, usize::from(upvalue.index))?
        } else {
            parent_upvalues
                .get(usize::from(upvalue.index))
                .copied()
                .ok_or(RuntimeErrorKind::UpvalueOutOfBounds {
                    index: usize::from(upvalue.index),
                })?
        };
        captured.push(value);
    }
    Ok(captured)
}

fn execute_vararg(
    thread: &mut LuaThread,
    instr: Instr,
    varargs: &[Value],
) -> RuntimeResult<Option<usize>> {
    let base = usize::from(instr.a());
    let count = if instr.b() == 0 {
        varargs.len()
    } else {
        instr.b() as usize
    };

    for index in 0..count {
        let value = varargs.get(index).copied().unwrap_or_else(Value::nil);
        set_register(thread, base + index, value)?;
    }

    Ok((instr.b() == 0).then_some(base + count))
}

pub(super) fn execute_tbc(
    thread: &LuaThread,
    context: &mut ExecutionContext<'_>,
    instr: Instr,
) -> RuntimeResult<()> {
    let register = usize::from(instr.a());
    let value = register_value_for_close(thread, register)?;
    if value.is_nil() || value.as_bool() == Some(false) {
        return Ok(());
    }

    let Some(metamethod) =
        context
            .tables
            .metamethod_for_value(value, "__close", context.strings)?
    else {
        return Err(RuntimeErrorKind::NonClosableValue.into());
    };
    if metamethod.as_closure_index().is_none() {
        return Err(RuntimeErrorKind::UnsupportedCloseMetamethod.into());
    }
    context.to_be_closed.push(register);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn close_to_count(
    thread: &LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
    to_be_closed: &mut Vec<usize>,
    keep_count: usize,
) -> RuntimeResult<()> {
    while to_be_closed.len() > keep_count {
        let register = to_be_closed.pop().expect("to-be-closed list is non-empty");
        call_close_metamethod(
            thread, closures, tables, strings, natives, globals, register,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn close_to_base(
    thread: &LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
    to_be_closed: &mut Vec<usize>,
    base: usize,
) -> RuntimeResult<()> {
    while to_be_closed
        .last()
        .is_some_and(|register| *register >= base)
    {
        let register = to_be_closed.pop().expect("to-be-closed list is non-empty");
        call_close_metamethod(
            thread, closures, tables, strings, natives, globals, register,
        )?;
    }
    Ok(())
}

fn call_close_metamethod(
    thread: &LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
    register: usize,
) -> RuntimeResult<()> {
    let value = register_value_for_close(thread, register)?;
    if value.is_nil() || value.as_bool() == Some(false) {
        return Ok(());
    }
    let Some(metamethod) = tables.metamethod_for_value(value, "__close", strings)? else {
        return Err(RuntimeErrorKind::NonClosableValue.into());
    };
    let Some(closure_index) = metamethod.as_closure_index() else {
        return Err(RuntimeErrorKind::UnsupportedCloseMetamethod.into());
    };
    call_closure(
        closures,
        closure_index as usize,
        &[value],
        tables,
        strings,
        natives,
        globals,
    )?;
    Ok(())
}

fn register_value_for_close(thread: &LuaThread, index: usize) -> RuntimeResult<Value> {
    register(thread, index)
}

fn callable_function(
    thread: &LuaThread,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    instr: Instr,
) -> RuntimeResult<CallableFunction> {
    let callee = register(thread, instr.a().into())?;
    if let Some(closure_index) = callee.as_closure_index() {
        return Ok(CallableFunction::Lua {
            closure_index: closure_index as usize,
            args: collect_call_args(thread, instr)?,
        });
    }
    if let Some(native_index) = callee.as_native_function_index() {
        return Ok(CallableFunction::Native {
            native_index: native_index as usize,
            args: collect_call_args(thread, instr)?,
        });
    }

    let Some(metamethod) = tables.metamethod_for_value(callee, "__call", strings)? else {
        return Err(RuntimeErrorKind::NonCallableValue.into());
    };
    let mut args = Vec::with_capacity(instr.b() as usize);
    args.push(callee);
    args.extend(collect_call_args(thread, instr)?);
    if let Some(closure_index) = metamethod.as_closure_index() {
        return Ok(CallableFunction::Lua {
            closure_index: closure_index as usize,
            args,
        });
    }
    if let Some(native_index) = metamethod.as_native_function_index() {
        return Ok(CallableFunction::Native {
            native_index: native_index as usize,
            args,
        });
    }
    Err(RuntimeErrorKind::UnsupportedMetamethod { name: "__call" }.into())
}

fn call_function(
    callable: CallableFunction,
    context: &mut ExecutionContext<'_>,
) -> RuntimeResult<Vec<Value>> {
    match callable {
        CallableFunction::Lua {
            closure_index,
            args,
        } => call_closure(
            context.closures,
            closure_index,
            &args,
            context.tables,
            context.strings,
            context.natives,
            context.globals,
        ),
        CallableFunction::Native { native_index, args } => {
            let function = context.natives.get(native_index).ok_or(
                RuntimeErrorKind::NativeFunctionOutOfBounds {
                    index: native_index,
                },
            )?;
            let mut native_context = NativeContext {
                tables: context.tables,
                strings: context.strings,
            };
            function(&mut native_context, &args)
        }
    }
}

#[cfg(test)]
fn execute_call(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<usize>> {
    let callable = callable_function(thread, tables, strings, instr)?;
    let mut to_be_closed = Vec::new();
    let mut context = ExecutionContext {
        closures,
        tables,
        strings,
        natives,
        globals,
        to_be_closed: &mut to_be_closed,
    };
    let returns = call_function(callable, &mut context)?;
    write_call_returns(thread, instr, &returns)
}

fn write_call_returns(
    thread: &mut LuaThread,
    instr: Instr,
    returns: &[Value],
) -> RuntimeResult<Option<usize>> {
    let top = write_values(thread, usize::from(instr.a()), returns, instr.c())?;
    Ok((instr.c() == 0).then_some(top))
}

fn write_values(
    thread: &mut LuaThread,
    base: usize,
    values: &[Value],
    result_count: u32,
) -> RuntimeResult<usize> {
    let count = if result_count == 0 {
        values.len()
    } else {
        result_count as usize
    };
    for index in 0..count {
        let value = values.get(index).copied().unwrap_or_else(Value::nil);
        set_register(thread, base + index, value)?;
    }
    Ok(base + count)
}

fn call_closure(
    closures: &mut Vec<RuntimeClosure>,
    closure_index: usize,
    args: &[Value],
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    natives: &RuntimeNatives,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Vec<Value>> {
    let closure = closures
        .get(closure_index)
        .cloned()
        .ok_or(RuntimeErrorKind::NonCallableValue)?;
    let mut to_be_closed = Vec::new();
    let mut context = ExecutionContext {
        closures,
        tables,
        strings,
        natives,
        globals,
        to_be_closed: &mut to_be_closed,
    };
    execute_proto_with_upvalues(&closure.proto, &closure.upvalues, args, &mut context, false)
        .map_err(|mut error| {
            error.push_trace_frame(trace_frame(&closure.proto));
            error
        })
}

fn trace_frame(proto: &Proto) -> TraceFrame {
    TraceFrame::new(
        proto.debug.source_name.as_deref(),
        proto.debug.source_name.as_deref(),
    )
}

fn collect_call_args(thread: &LuaThread, instr: Instr) -> RuntimeResult<Vec<Value>> {
    let count = instr.b().saturating_sub(1);
    let mut args = Vec::with_capacity(count as usize);
    let base = usize::from(instr.a()) + 1;
    for index in 0..count {
        args.push(register(thread, base + index as usize)?);
    }
    Ok(args)
}

fn collect_returns(
    thread: &LuaThread,
    instr: Instr,
    dynamic_top: usize,
) -> RuntimeResult<Vec<Value>> {
    let base = usize::from(instr.a());
    let count = if instr.b() == 0 {
        dynamic_top.saturating_sub(base)
    } else {
        instr.b() as usize
    };
    let mut values = Vec::with_capacity(count);
    for index in base..base + count {
        values.push(register(thread, index)?);
    }
    Ok(values)
}

fn register(thread: &LuaThread, index: usize) -> RuntimeResult<Value> {
    thread
        .stack_value(index)
        .ok_or_else(|| RuntimeErrorKind::RegisterOutOfBounds { register: index }.into())
}

fn set_register(thread: &mut LuaThread, index: usize, value: Value) -> RuntimeResult<()> {
    if thread.set_stack_value(index, value) {
        Ok(())
    } else {
        Err(RuntimeErrorKind::RegisterOutOfBounds { register: index }.into())
    }
}

#[cfg(test)]
mod metamethod_tests;
#[cfg(test)]
mod tests;
