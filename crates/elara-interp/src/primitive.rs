//! Primitive bytecode execution.

use elara_bytecode::{Instr, Op, Proto, VerifyError, verify_proto};
use elara_core::{
    GcArena, LuaError, LuaFloat, LuaInteger, LuaThread, StringInterner, Table, TraceFrame, Value,
};

mod global;
mod loops;
mod metamethod;
mod table;

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

/// Runtime-owned string storage for primitive execution output.
#[derive(Default)]
pub struct RuntimeStrings {
    arena: GcArena,
    interner: StringInterner,
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
                format!("attempt to perform '{}' on a non-number value", op.mnemonic())
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
    verify_proto(proto)
        .map_err(RuntimeErrorKind::Verification)
        .map_err(RuntimeError::from)?;
    let mut closures = Vec::new();
    let mut tables = RuntimeTables::new();
    let mut strings = RuntimeStrings::new();
    let global_table = tables.push_table(Table::new());
    let mut globals = RuntimeGlobals::new(global_table);
    let values = execute_proto_with_upvalues(
        proto,
        &[globals.value()],
        &[],
        &mut closures,
        &mut tables,
        &mut strings,
        &mut globals,
    )?;
    Ok(RuntimeOutput {
        values,
        tables,
        strings,
    })
}

fn execute_proto_with_upvalues(
    proto: &Proto,
    upvalues: &[Value],
    varargs: &[Value],
    closures: &mut Vec<RuntimeClosure>,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Vec<Value>> {
    let mut thread = LuaThread::new();
    for _ in 0..proto.max_stack {
        thread.push_value(Value::nil());
    }
    let mut dynamic_top = 0;

    let mut pc = 0;
    while pc < proto.code.len() {
        let instr = proto.code[pc];
        pc += 1;

        match instr.op() {
            Op::Move => {
                let value = register(&thread, instr.b() as usize)?;
                set_register(&mut thread, instr.a().into(), value)?;
            }
            Op::LoadNil => set_register(&mut thread, instr.a().into(), Value::nil())?,
            Op::LoadBool => {
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::boolean(instr.b() != 0),
                )?;
            }
            Op::LoadInt => {
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::integer(LuaInteger::from(instr.b())),
                )?;
            }
            Op::LoadFloat => {
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::float(LuaFloat::from(instr.b())),
                )?;
            }
            Op::LoadK => {
                let constant = proto.constants.get(instr.bx() as usize).copied().ok_or(
                    RuntimeErrorKind::ConstantOutOfBounds {
                        index: instr.bx() as usize,
                    },
                )?;
                set_register(&mut thread, instr.a().into(), constant)?;
            }
            Op::LoadString => {
                let string = proto.string_constants.get(instr.bx() as usize).ok_or(
                    RuntimeErrorKind::StringOutOfBounds {
                        index: instr.bx() as usize,
                    },
                )?;
                let value = strings.intern_short_value(string);
                set_register(&mut thread, instr.a().into(), value)?;
            }
            Op::GetEnv => {
                let name = string_constant(proto, instr)?;
                execute_get_env(&mut thread, instr, name, globals, strings, tables)?;
            }
            Op::SetEnv => {
                let name = string_constant(proto, instr)?;
                execute_set_env(&thread, instr, name, globals, strings, tables)?;
            }
            Op::DeclGlobal => {
                let name = string_constant(proto, instr)?;
                execute_decl_global(&mut thread, instr, name, strings)?;
            }
            Op::NewTable => execute_new_table(&mut thread, instr, tables)?,
            Op::GetTable => {
                execute_get_table(&mut thread, closures, instr, tables, strings, globals)?
            }
            Op::SetTable => {
                execute_set_table(&mut thread, closures, instr, tables, strings, globals)?
            }
            Op::GetIndex => {
                execute_get_index(&mut thread, closures, instr, tables, strings, globals)?
            }
            Op::SetIndex => {
                execute_set_index(&mut thread, closures, instr, tables, strings, globals)?
            }
            Op::GetUpvalue => {
                let value = upvalues.get(instr.b() as usize).copied().ok_or(
                    RuntimeErrorKind::UpvalueOutOfBounds {
                        index: instr.b() as usize,
                    },
                )?;
                set_register(&mut thread, instr.a().into(), value)?;
            }
            Op::Closure => {
                let child_index = instr.bx() as usize;
                let child = proto
                    .children
                    .get(child_index)
                    .cloned()
                    .ok_or(RuntimeErrorKind::ChildOutOfBounds { index: child_index })?;
                let closure_index = closures.len();
                closures.push(RuntimeClosure {
                    proto: child.clone(),
                    upvalues: Vec::new(),
                });
                set_register(
                    &mut thread,
                    instr.a().into(),
                    Value::closure_index(closure_index as u32),
                )?;
                let captured = capture_upvalues(&child, &thread, upvalues)?;
                closures[closure_index].upvalues = captured;
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::IDiv | Op::Mod | Op::Pow | Op::Unm => {
                execute_arithmetic(&mut thread, closures, instr, tables, strings, globals)?
            }
            Op::Len => execute_len(&mut thread, closures, instr, tables, strings, globals)?,
            Op::Concat => execute_concat(&mut thread, closures, instr, tables, strings, globals)?,
            Op::Eq | Op::Lt | Op::Le => {
                execute_comparison(&mut thread, closures, instr, tables, strings, globals)?;
            }
            Op::Jmp => pc = jump_target(pc, instr)?,
            Op::ForPrep => {
                if prepare_numeric_for(&mut thread, instr)? {
                    pc = jump_target(pc, instr)?;
                }
            }
            Op::ForLoop => {
                if execute_numeric_for_loop(&mut thread, instr)? {
                    pc = jump_target(pc, instr)?;
                }
            }
            Op::TForPrep => pc = jump_target(pc, instr)?,
            Op::TForCall => {
                execute_generic_for_call(&mut thread, closures, instr, tables, strings, globals)?;
            }
            Op::TForLoop => {
                if execute_generic_for_loop(&mut thread, instr)? {
                    pc = jump_target(pc, instr)?;
                }
            }
            Op::Test => {
                let value = register(&thread, instr.a().into())?;
                if is_truthy(value) != (instr.b() != 0) {
                    pc += 1;
                }
            }
            Op::Vararg => {
                if let Some(top) = execute_vararg(&mut thread, instr, varargs)? {
                    dynamic_top = top;
                }
            }
            Op::VarargTable => execute_vararg_table(&mut thread, instr, varargs, tables)?,
            Op::Call => {
                if let Some(top) =
                    execute_call(&mut thread, closures, instr, tables, strings, globals)?
                {
                    dynamic_top = top;
                }
            }
            Op::Return => return collect_returns(&thread, instr, dynamic_top),
            op => return Err(RuntimeErrorKind::UnsupportedOpcode { op }.into()),
        }
    }

    Ok(Vec::new())
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
struct RuntimeClosure {
    proto: Proto,
    upvalues: Vec<Value>,
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

fn execute_call(
    thread: &mut LuaThread,
    closures: &mut Vec<RuntimeClosure>,
    instr: Instr,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Option<usize>> {
    let callee = register(thread, instr.a().into())?;
    let (closure_index, args) = if let Some(closure_index) = callee.as_closure_index() {
        (closure_index as usize, collect_call_args(thread, instr)?)
    } else {
        let Some(metamethod) = tables.metamethod_for_value(callee, "__call", strings)? else {
            return Err(RuntimeErrorKind::NonCallableValue.into());
        };
        let Some(closure_index) = metamethod.as_closure_index() else {
            return Err(RuntimeErrorKind::UnsupportedMetamethod { name: "__call" }.into());
        };
        let mut args = Vec::with_capacity(instr.b() as usize);
        args.push(callee);
        args.extend(collect_call_args(thread, instr)?);
        (closure_index as usize, args)
    };
    let returns = call_closure(closures, closure_index, &args, tables, strings, globals)?;

    let base = usize::from(instr.a());
    let count = if instr.c() == 0 {
        returns.len()
    } else {
        instr.c() as usize
    };

    for index in 0..count {
        let value = returns.get(index).copied().unwrap_or_else(Value::nil);
        set_register(thread, base + index, value)?;
    }

    Ok((instr.c() == 0).then_some(base + count))
}

fn call_closure(
    closures: &mut Vec<RuntimeClosure>,
    closure_index: usize,
    args: &[Value],
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    globals: &mut RuntimeGlobals,
) -> RuntimeResult<Vec<Value>> {
    let closure = closures
        .get(closure_index)
        .cloned()
        .ok_or(RuntimeErrorKind::NonCallableValue)?;
    execute_proto_with_upvalues(
        &closure.proto,
        &closure.upvalues,
        args,
        closures,
        tables,
        strings,
        globals,
    )
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
