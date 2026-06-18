//! Debug metadata materialization for primitive execution.

use elara_bytecode::Proto;
use elara_core::{LuaInteger, LuaThread, Table, Value};

use super::{
    RuntimeClosure, RuntimeErrorKind, RuntimeNatives, RuntimeResult, RuntimeStrings, RuntimeTables,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RuntimeDebugFrameKind {
    Main,
    Lua,
    Native,
}

#[derive(Clone, Debug)]
pub(super) struct RuntimeDebugFrame {
    proto: Option<Proto>,
    function: Value,
    kind: RuntimeDebugFrameKind,
    locals: Vec<Value>,
    pub(super) current_pc: Option<usize>,
}

impl RuntimeDebugFrame {
    pub(super) fn new(proto: Proto, function: Option<Value>, kind: RuntimeDebugFrameKind) -> Self {
        Self {
            proto: Some(proto),
            function: function.unwrap_or_else(Value::nil),
            kind,
            locals: Vec::new(),
            current_pc: None,
        }
    }

    pub(super) fn native(native_index: usize) -> Self {
        Self {
            proto: None,
            function: Value::native_function_index(
                u32::try_from(native_index).expect("native function index must fit in u32"),
            ),
            kind: RuntimeDebugFrameKind::Native,
            locals: Vec::new(),
            current_pc: None,
        }
    }

    pub(super) fn capture_locals(&mut self, thread: &LuaThread) {
        self.locals.clear();
        self.locals.reserve(thread.stack_len());
        for index in 0..thread.stack_len() {
            if let Some(value) = thread.stack_value(index) {
                self.locals.push(value);
            }
        }
    }
}

pub(super) fn info_for_level(
    level: i64,
    options: &[u8],
    closures: &[RuntimeClosure],
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
    debug_frames: &[RuntimeDebugFrame],
) -> RuntimeResult<Value> {
    let Some(level) = usize::try_from(level).ok() else {
        return Ok(Value::nil());
    };
    let Some(index) = debug_frames.len().checked_sub(level + 1) else {
        return Ok(Value::nil());
    };
    let frame = &debug_frames[index];
    info_for_frame(frame, options, closures, tables, strings)
}

pub(super) fn info_for_function(
    function: Value,
    options: &[u8],
    closures: &[RuntimeClosure],
    natives: &RuntimeNatives,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Value> {
    if let Some(index) = function.as_closure_index() {
        let Some(closure) = closures.get(index as usize) else {
            return Ok(Value::nil());
        };
        let frame = RuntimeDebugFrame::new(
            closure.proto.clone(),
            Some(function),
            RuntimeDebugFrameKind::Lua,
        );
        return info_for_frame(&frame, options, closures, tables, strings);
    }
    if let Some(index) = function.as_native_function_index() {
        if natives.get(index as usize).is_none() {
            return Ok(Value::nil());
        }
        let frame = RuntimeDebugFrame::native(index as usize);
        return info_for_frame(&frame, options, closures, tables, strings);
    }
    Ok(Value::nil())
}

pub(super) fn traceback(
    message: Option<&[u8]>,
    level: i64,
    strings: &mut RuntimeStrings,
    debug_frames: &[RuntimeDebugFrame],
) -> RuntimeResult<Value> {
    let mut output = Vec::new();
    if let Some(message) = message {
        output.extend_from_slice(message);
        output.push(b'\n');
    }
    output.extend_from_slice(b"stack traceback:");
    let Some(mut level) = usize::try_from(level).ok() else {
        return Ok(strings.intern_value(output));
    };
    while let Some(index) = debug_frames.len().checked_sub(level + 1) {
        let frame = &debug_frames[index];
        output.extend_from_slice(b"\n\t");
        output.extend_from_slice(frame.source());
        let current_line = frame.current_line();
        if current_line > 0 {
            output.push(b':');
            output.extend_from_slice(current_line.to_string().as_bytes());
        }
        output.extend_from_slice(b": in ");
        output.extend_from_slice(frame.traceback_label());
        level += 1;
    }
    Ok(strings.intern_value(output))
}

pub(super) fn get_local(
    level: i64,
    local: i64,
    strings: &mut RuntimeStrings,
    debug_frames: &[RuntimeDebugFrame],
) -> RuntimeResult<Option<(Value, Value)>> {
    let frame_index = frame_index_for_level(level, debug_frames)?;
    let frame = &debug_frames[frame_index];
    let Some((register, name)) = active_local(frame, local) else {
        return Ok(None);
    };
    let Some(value) = frame.locals.get(register).copied() else {
        return Ok(None);
    };
    Ok(Some((strings.intern_value(name), value)))
}

pub(super) fn get_local_name_for_function(
    function: Value,
    local: i64,
    closures: &[RuntimeClosure],
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<Value>> {
    let Some(local_index) = local
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return Ok(None);
    };
    let Some(closure_index) = function.as_closure_index() else {
        return Ok(None);
    };
    let Some(closure) = closures.get(closure_index as usize) else {
        return Ok(None);
    };
    if local_index >= usize::from(closure.proto.params) {
        return Ok(None);
    }
    let Some(local) = closure
        .proto
        .debug
        .local_vars
        .iter()
        .find(|local| usize::from(local.register) == local_index)
    else {
        return Ok(None);
    };
    Ok(Some(strings.intern_value(local.name.as_bytes())))
}

pub(super) fn set_local(
    level: i64,
    local: i64,
    value: Value,
    strings: &mut RuntimeStrings,
    debug_frames: &mut [RuntimeDebugFrame],
    current_thread: Option<&mut LuaThread>,
    current_frame_index: Option<usize>,
) -> RuntimeResult<Option<Value>> {
    let frame_index = frame_index_for_level(level, debug_frames)?;
    if Some(frame_index) != current_frame_index {
        return Ok(None);
    }
    let Some((register, name)) = active_local(&debug_frames[frame_index], local) else {
        return Ok(None);
    };
    let name = name.to_vec();
    let Some(thread) = current_thread else {
        return Ok(None);
    };
    if !thread.set_stack_value(register, value) {
        return Ok(None);
    }
    if let Some(slot) = debug_frames[frame_index].locals.get_mut(register) {
        *slot = value;
    }
    Ok(Some(strings.intern_value(name)))
}

fn frame_index_for_level(level: i64, debug_frames: &[RuntimeDebugFrame]) -> RuntimeResult<usize> {
    let Some(level) = usize::try_from(level).ok() else {
        return Err(RuntimeErrorKind::NativeFunctionError {
            message: "level out of range".into(),
            error_object: None,
        }
        .into());
    };
    debug_frames.len().checked_sub(level + 1).ok_or_else(|| {
        RuntimeErrorKind::NativeFunctionError {
            message: "level out of range".into(),
            error_object: None,
        }
        .into()
    })
}

fn active_local(frame: &RuntimeDebugFrame, local: i64) -> Option<(usize, &[u8])> {
    let local_index = local
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())?;
    let proto = frame.proto.as_ref()?;
    let pc = frame.current_pc.and_then(|pc| u32::try_from(pc).ok())?;
    let local = proto
        .debug
        .local_vars
        .iter()
        .filter(|local| local.start_pc <= pc && pc < local.end_pc)
        .nth(local_index)?;
    Some((usize::from(local.register), local.name.as_bytes()))
}

pub(super) fn get_upvalue(
    function: Value,
    index: i64,
    closures: &[RuntimeClosure],
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<(Value, Value)>> {
    let Some(index) = index
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return Ok(None);
    };
    let Some(closure_index) = function.as_closure_index() else {
        return Ok(None);
    };
    let Some(closure) = closures.get(closure_index as usize) else {
        return Ok(None);
    };
    let Some(upvalue) = closure.upvalues.get(index) else {
        return Ok(None);
    };
    let value = upvalue.get();
    let name = closure
        .proto
        .upvalues
        .get(index)
        .and_then(|upvalue| upvalue.name.as_deref())
        .unwrap_or("?");
    Ok(Some((strings.intern_value(name), value)))
}

pub(super) fn set_upvalue(
    function: Value,
    index: i64,
    value: Value,
    closures: &mut [RuntimeClosure],
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Option<Value>> {
    let Some(index) = index
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return Ok(None);
    };
    let Some(closure_index) = function.as_closure_index() else {
        return Ok(None);
    };
    let Some(closure) = closures.get_mut(closure_index as usize) else {
        return Ok(None);
    };
    let Some(upvalue) = closure.upvalues.get(index) else {
        return Ok(None);
    };
    upvalue.set(value);
    let name = closure
        .proto
        .upvalues
        .get(index)
        .and_then(|upvalue| upvalue.name.as_deref())
        .unwrap_or("?");
    Ok(Some(strings.intern_value(name)))
}

pub(super) fn upvalue_id(
    function: Value,
    index: i64,
    closures: &[RuntimeClosure],
) -> RuntimeResult<Option<Value>> {
    let Some(index) = index
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return Ok(None);
    };
    let Some(closure_index) = function.as_closure_index() else {
        return Ok(None);
    };
    let Some(closure) = closures.get(closure_index as usize) else {
        return Ok(None);
    };
    let Some(upvalue) = closure.upvalues.get(index) else {
        return Ok(None);
    };
    Ok(Some(Value::light_user_data(upvalue.identity())))
}

pub(super) fn join_upvalue(
    target_function: Value,
    target_index: i64,
    source_function: Value,
    source_index: i64,
    closures: &mut [RuntimeClosure],
) -> RuntimeResult<bool> {
    let Some(target_index) = target_index
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return Ok(false);
    };
    let Some(source_index) = source_index
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
    else {
        return Ok(false);
    };
    let Some(target_closure_index) = target_function.as_closure_index() else {
        return Ok(false);
    };
    let Some(source_closure_index) = source_function.as_closure_index() else {
        return Ok(false);
    };
    let Some(source_upvalue) = closures
        .get(source_closure_index as usize)
        .and_then(|closure| closure.upvalues.get(source_index))
        .cloned()
    else {
        return Ok(false);
    };
    let Some(target_upvalue) = closures
        .get_mut(target_closure_index as usize)
        .and_then(|closure| closure.upvalues.get_mut(target_index))
    else {
        return Ok(false);
    };
    *target_upvalue = source_upvalue;
    Ok(true)
}

fn info_for_frame(
    frame: &RuntimeDebugFrame,
    options: &[u8],
    _closures: &[RuntimeClosure],
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<Value> {
    validate_options(options)?;
    let mut table = Table::new();
    for option in options {
        match *option {
            b'S' => set_source_fields(&mut table, frame, strings)?,
            b'l' => set_integer_field(&mut table, strings, "currentline", frame.current_line())?,
            b'u' => set_parameter_fields(&mut table, frame, strings)?,
            b't' => set_tail_fields(&mut table, strings)?,
            b'n' => set_name_fields(&mut table, strings)?,
            b'r' => set_transfer_fields(&mut table, strings)?,
            b'f' => set_value_field(&mut table, strings, "func", frame.function)?,
            b'L' => set_active_lines_field(&mut table, frame, tables, strings)?,
            _ => unreachable!("debug.getinfo options are validated before table materialization"),
        }
    }
    Ok(Value::table_index(tables.push_table(table)))
}

fn validate_options(options: &[u8]) -> RuntimeResult<()> {
    for option in options {
        match *option {
            b'S' | b'l' | b'u' | b't' | b'n' | b'r' | b'f' | b'L' => {}
            option => {
                let display = char::from(option);
                return Err(RuntimeErrorKind::NativeFunctionError {
                    message: format!("invalid option '{display}'").into_boxed_str(),
                    error_object: None,
                }
                .into());
            }
        }
    }
    Ok(())
}

impl RuntimeDebugFrame {
    fn current_line(&self) -> LuaInteger {
        let Some(proto) = &self.proto else {
            return -1;
        };
        let Some(pc) = self.current_pc else {
            return -1;
        };
        let line = proto.debug.line_info.get(pc).copied().unwrap_or_default();
        if line == 0 {
            -1
        } else {
            LuaInteger::from(line)
        }
    }

    fn source(&self) -> &[u8] {
        self.proto
            .as_ref()
            .and_then(|proto| proto.debug.source_name.as_deref())
            .map_or(b"=?" as &[u8], str::as_bytes)
    }

    fn what(&self) -> &'static [u8] {
        match self.kind {
            RuntimeDebugFrameKind::Main => b"main",
            RuntimeDebugFrameKind::Lua => b"Lua",
            RuntimeDebugFrameKind::Native => b"C",
        }
    }

    fn traceback_label(&self) -> &'static [u8] {
        match self.kind {
            RuntimeDebugFrameKind::Main => b"main chunk",
            RuntimeDebugFrameKind::Lua => b"function",
            RuntimeDebugFrameKind::Native => b"function",
        }
    }

    fn line_defined(&self) -> LuaInteger {
        match self.kind {
            RuntimeDebugFrameKind::Native => -1,
            RuntimeDebugFrameKind::Main | RuntimeDebugFrameKind::Lua => 0,
        }
    }

    fn last_line_defined(&self) -> LuaInteger {
        let Some(proto) = &self.proto else {
            return -1;
        };
        proto
            .debug
            .line_info
            .iter()
            .copied()
            .filter(|line| *line != 0)
            .max()
            .map_or(0, LuaInteger::from)
    }

    fn nups(&self) -> LuaInteger {
        self.proto.as_ref().map_or(0, |proto| {
            LuaInteger::try_from(proto.upvalues.len()).unwrap_or(0)
        })
    }

    fn nparams(&self) -> LuaInteger {
        self.proto
            .as_ref()
            .map_or(0, |proto| LuaInteger::from(proto.params))
    }

    fn is_vararg(&self) -> bool {
        matches!(self.kind, RuntimeDebugFrameKind::Main)
            || self.proto.as_ref().is_none_or(|proto| proto.is_vararg)
    }
}

fn set_source_fields(
    table: &mut Table,
    frame: &RuntimeDebugFrame,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let source = frame.source();
    set_string_field(table, strings, "source", source)?;
    set_string_field(table, strings, "short_src", source)?;
    set_integer_field(table, strings, "linedefined", frame.line_defined())?;
    set_integer_field(table, strings, "lastlinedefined", frame.last_line_defined())?;
    set_string_field(table, strings, "what", frame.what())
}

fn set_parameter_fields(
    table: &mut Table,
    frame: &RuntimeDebugFrame,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    set_integer_field(table, strings, "nups", frame.nups())?;
    set_integer_field(table, strings, "nparams", frame.nparams())?;
    set_value_field(
        table,
        strings,
        "isvararg",
        Value::boolean(frame.is_vararg()),
    )
}

fn set_tail_fields(table: &mut Table, strings: &mut RuntimeStrings) -> RuntimeResult<()> {
    set_value_field(table, strings, "istailcall", Value::boolean(false))?;
    set_integer_field(table, strings, "extraargs", 0)
}

fn set_name_fields(table: &mut Table, strings: &mut RuntimeStrings) -> RuntimeResult<()> {
    set_string_field(table, strings, "namewhat", b"")
}

fn set_transfer_fields(table: &mut Table, strings: &mut RuntimeStrings) -> RuntimeResult<()> {
    set_integer_field(table, strings, "ftransfer", 0)?;
    set_integer_field(table, strings, "ntransfer", 0)
}

fn set_active_lines_field(
    table: &mut Table,
    frame: &RuntimeDebugFrame,
    tables: &mut RuntimeTables,
    strings: &mut RuntimeStrings,
) -> RuntimeResult<()> {
    let mut active_lines = Table::new();
    if let Some(proto) = &frame.proto {
        for line in proto
            .debug
            .line_info
            .iter()
            .copied()
            .filter(|line| *line != 0)
        {
            let line = Value::integer(LuaInteger::from(line));
            if !active_lines.raw_set_value(line, Value::boolean(true)) {
                return Err(RuntimeErrorKind::InvalidTableKey.into());
            }
        }
    }
    let active_lines = Value::table_index(tables.push_table(active_lines));
    set_value_field(table, strings, "activelines", active_lines)
}

fn set_string_field(
    table: &mut Table,
    strings: &mut RuntimeStrings,
    key: &str,
    value: &[u8],
) -> RuntimeResult<()> {
    let value = strings.intern_value(value);
    set_value_field(table, strings, key, value)
}

fn set_integer_field(
    table: &mut Table,
    strings: &mut RuntimeStrings,
    key: &str,
    value: LuaInteger,
) -> RuntimeResult<()> {
    set_value_field(table, strings, key, Value::integer(value))
}

fn set_value_field(
    table: &mut Table,
    strings: &mut RuntimeStrings,
    key: &str,
    value: Value,
) -> RuntimeResult<()> {
    let key = strings.intern_short_value(key);
    if table.raw_set_value(key, value) {
        Ok(())
    } else {
        Err(RuntimeErrorKind::InvalidTableKey.into())
    }
}
