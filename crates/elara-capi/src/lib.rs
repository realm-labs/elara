#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

//! Optional Lua 5.5 C API layer for Elara.
//!
//! This crate is the optional source-compatible C API surface for the current
//! Lua target only. It should be implemented on top of the safe Rust API where
//! practical.
//!
//! It must not introduce compatibility branches for old Lua C APIs.
//! Stack API entrypoints return neutral C API values for null states and invalid
//! indices. The initial protected C-call boundary catches Rust panics from
//! `extern "C-unwind"` callbacks and converts them into Lua-style runtime
//! errors.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::panic::{self, AssertUnwindSafe};
use std::ptr;

const LUA_MULTRET: c_int = -1;
const LUA_OK: c_int = 0;
const LUA_ERRRUN: c_int = 2;

const LUA_TNONE: c_int = -1;
const LUA_TNIL: c_int = 0;
const LUA_TBOOLEAN: c_int = 1;
const LUA_TLIGHTUSERDATA: c_int = 2;
const LUA_TNUMBER: c_int = 3;
const LUA_TSTRING: c_int = 4;
const LUA_TFUNCTION: c_int = 6;
const LUA_REGISTRYINDEX: c_int = -1_000_000;

const LUA_TYPENAMES: [&[u8]; 10] = [
    b"no value\0",
    b"nil\0",
    b"boolean\0",
    b"userdata\0",
    b"number\0",
    b"string\0",
    b"table\0",
    b"function\0",
    b"userdata\0",
    b"thread\0",
];

pub type lua_Integer = i64;
pub type lua_Unsigned = u64;
pub type lua_Number = f64;
pub type lua_CFunction = Option<unsafe extern "C-unwind" fn(*mut lua_State) -> c_int>;
pub type lua_KContext = isize;
pub type lua_KFunction =
    Option<unsafe extern "C-unwind" fn(*mut lua_State, c_int, lua_KContext) -> c_int>;
pub type lua_Alloc =
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void>;

/// Opaque current-version Lua C API state.
#[repr(C)]
pub struct lua_State {
    stack: Vec<CValue>,
    call_bases: Vec<usize>,
    _alloc: lua_Alloc,
    _alloc_ud: *mut c_void,
}

#[derive(Clone)]
enum CValue {
    Nil,
    Boolean(bool),
    Integer(lua_Integer),
    Number(lua_Number),
    String(Vec<u8>),
    LightUserData(*mut c_void),
    CFunction(lua_CFunction),
}

impl CValue {
    fn type_tag(&self) -> c_int {
        match self {
            Self::Nil => LUA_TNIL,
            Self::Boolean(_) => LUA_TBOOLEAN,
            Self::Integer(_) | Self::Number(_) => LUA_TNUMBER,
            Self::String(_) => LUA_TSTRING,
            Self::LightUserData(_) => LUA_TLIGHTUSERDATA,
            Self::CFunction(_) => LUA_TFUNCTION,
        }
    }

    fn as_number(&self) -> Option<lua_Number> {
        match self {
            Self::Integer(value) => Some(*value as lua_Number),
            Self::Number(value) => Some(*value),
            Self::String(bytes) => string_payload(bytes)
                .and_then(|payload| std::str::from_utf8(payload).ok())
                .and_then(|payload| payload.trim().parse().ok()),
            _ => None,
        }
    }

    fn as_integer(&self) -> Option<lua_Integer> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Number(value)
                if value.is_finite()
                    && value.fract() == 0.0
                    && *value >= lua_Integer::MIN as lua_Number
                    && *value <= lua_Integer::MAX as lua_Number =>
            {
                Some(*value as lua_Integer)
            }
            Self::String(bytes) => string_payload(bytes)
                .and_then(|payload| std::str::from_utf8(payload).ok())
                .and_then(|payload| payload.trim().parse().ok()),
            _ => None,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_newstate(f: lua_Alloc, ud: *mut c_void) -> *mut lua_State {
    if f.is_none() {
        return ptr::null_mut();
    }

    Box::into_raw(Box::new(lua_State {
        stack: Vec::new(),
        call_bases: Vec::new(),
        _alloc: f,
        _alloc_ud: ud,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_close(state: *mut lua_State) {
    if !state.is_null() {
        // SAFETY: `state` was returned by `lua_newstate` and this function
        // consumes that allocation exactly once.
        unsafe { drop(Box::from_raw(state)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_newthread(_state: *mut lua_State) -> *mut lua_State {
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_gettop(state: *mut lua_State) -> c_int {
    with_state(state, |state| {
        usize_to_c_int(state.stack.len().saturating_sub(current_base(state)))
    })
    .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_settop(state: *mut lua_State, idx: c_int) {
    let Some(state) = state_mut(state) else {
        return;
    };

    if idx >= 0 {
        let target_len = current_base(state).saturating_add(idx as usize);
        resize_stack(state, target_len);
        return;
    }

    let next_top = state.stack.len() as isize + idx as isize + 1;
    state
        .stack
        .truncate(next_top.max(current_base(state) as isize) as usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushvalue(state: *mut lua_State, idx: c_int) {
    let Some(state) = state_mut(state) else {
        return;
    };
    let value = stack_index(state, idx)
        .map(|index| state.stack[index].clone())
        .unwrap_or(CValue::Nil);
    push_stack(state, value);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_rotate(state: *mut lua_State, idx: c_int, n: c_int) {
    let Some(state) = state_mut(state) else {
        return;
    };
    let Some(start) = stack_index(state, idx) else {
        return;
    };
    let segment = &mut state.stack[start..];
    if segment.is_empty() {
        return;
    }

    let len = segment.len();
    if n >= 0 {
        segment.rotate_right((n as usize) % len);
    } else {
        segment.rotate_left((n.unsigned_abs() as usize) % len);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_copy(state: *mut lua_State, fromidx: c_int, toidx: c_int) {
    let Some(state) = state_mut(state) else {
        return;
    };
    let Some(to_index) = stack_index(state, toidx) else {
        return;
    };
    let value = stack_index(state, fromidx)
        .map(|from_index| state.stack[from_index].clone())
        .unwrap_or(CValue::Nil);
    state.stack[to_index] = value;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_checkstack(state: *mut lua_State, n: c_int) -> c_int {
    let Some(state) = state_mut(state) else {
        return 0;
    };
    if n < 0 {
        return 0;
    }
    c_bool(state.stack.try_reserve(n as usize).is_ok())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_type(state: *mut lua_State, idx: c_int) -> c_int {
    with_value(state, idx, CValue::type_tag).unwrap_or(LUA_TNONE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_typename(_state: *mut lua_State, tp: c_int) -> *const c_char {
    let name_index = if tp == LUA_TNONE {
        0
    } else if (LUA_TNIL..=8).contains(&tp) {
        (tp + 1) as usize
    } else {
        0
    };
    LUA_TYPENAMES[name_index].as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_isnumber(state: *mut lua_State, idx: c_int) -> c_int {
    c_bool(
        with_value(state, idx, CValue::as_number)
            .flatten()
            .is_some(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_isstring(state: *mut lua_State, idx: c_int) -> c_int {
    c_bool(matches!(
        with_value(state, idx, CValue::type_tag),
        Some(LUA_TSTRING | LUA_TNUMBER)
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_iscfunction(state: *mut lua_State, idx: c_int) -> c_int {
    c_bool(matches!(
        with_value(state, idx, CValue::type_tag),
        Some(LUA_TFUNCTION)
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_isinteger(state: *mut lua_State, idx: c_int) -> c_int {
    c_bool(with_value(state, idx, |value| matches!(value, CValue::Integer(_))).unwrap_or(false))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushnil(state: *mut lua_State) {
    push_value(state, CValue::Nil);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushnumber(state: *mut lua_State, n: lua_Number) {
    push_value(state, CValue::Number(n));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushinteger(state: *mut lua_State, n: lua_Integer) {
    push_value(state, CValue::Integer(n));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushlstring(
    state: *mut lua_State,
    s: *const c_char,
    len: usize,
) -> *const c_char {
    let Some(state) = state_mut(state) else {
        return ptr::null();
    };
    if s.is_null() {
        push_stack(state, CValue::Nil);
        return ptr::null();
    }

    // SAFETY: The C caller promises that `s` points to at least `len` readable
    // bytes. The bytes are copied into state-owned storage before returning.
    let payload = unsafe { std::slice::from_raw_parts(s.cast::<u8>(), len) };
    push_string_payload(state, payload)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushstring(state: *mut lua_State, s: *const c_char) -> *const c_char {
    let Some(state) = state_mut(state) else {
        return ptr::null();
    };
    if s.is_null() {
        push_stack(state, CValue::Nil);
        return ptr::null();
    }

    // SAFETY: `s` is a non-null C string pointer supplied by the caller.
    let bytes = unsafe { CStr::from_ptr(s) }.to_bytes();
    push_string_payload(state, bytes)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushcclosure(state: *mut lua_State, fn_: lua_CFunction, n: c_int) {
    let Some(state) = state_mut(state) else {
        return;
    };
    let upvalue_count = n.max(0) as usize;
    if upvalue_count > 0 {
        let new_len = state.stack.len().saturating_sub(upvalue_count);
        state.stack.truncate(new_len);
    }
    push_stack(state, CValue::CFunction(fn_));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushboolean(state: *mut lua_State, b: c_int) {
    push_value(state, CValue::Boolean(b != 0));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pushlightuserdata(state: *mut lua_State, p: *mut c_void) {
    push_value(state, CValue::LightUserData(p));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_tonumberx(
    state: *mut lua_State,
    idx: c_int,
    isnum: *mut c_int,
) -> lua_Number {
    let value = with_value(state, idx, CValue::as_number).flatten();
    set_out_bool(isnum, value.is_some());
    value.unwrap_or(0.0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_tointegerx(
    state: *mut lua_State,
    idx: c_int,
    isnum: *mut c_int,
) -> lua_Integer {
    let value = with_value(state, idx, CValue::as_integer).flatten();
    set_out_bool(isnum, value.is_some());
    value.unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_toboolean(state: *mut lua_State, idx: c_int) -> c_int {
    c_bool(
        with_value(state, idx, |value| {
            !matches!(value, CValue::Nil | CValue::Boolean(false))
        })
        .unwrap_or(false),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_tolstring(
    state: *mut lua_State,
    idx: c_int,
    len: *mut usize,
) -> *const c_char {
    let Some(state) = state_mut(state) else {
        set_out_len(len, 0);
        return ptr::null();
    };
    let Some(index) = stack_index(state, idx) else {
        set_out_len(len, 0);
        return ptr::null();
    };
    let CValue::String(bytes) = &state.stack[index] else {
        set_out_len(len, 0);
        return ptr::null();
    };

    let payload_len = bytes.len().saturating_sub(1);
    set_out_len(len, payload_len);
    bytes.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_tocfunction(state: *mut lua_State, idx: c_int) -> lua_CFunction {
    with_value(state, idx, |value| match value {
        CValue::CFunction(fn_) => *fn_,
        _ => None,
    })
    .flatten()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_touserdata(state: *mut lua_State, idx: c_int) -> *mut c_void {
    with_value(state, idx, |value| match value {
        CValue::LightUserData(ptr) => *ptr,
        _ => ptr::null_mut(),
    })
    .unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_topointer(state: *mut lua_State, idx: c_int) -> *const c_void {
    with_value(state, idx, |value| match value {
        CValue::String(bytes) => bytes.as_ptr().cast(),
        CValue::LightUserData(ptr) => ptr.cast_const(),
        CValue::CFunction(Some(fn_)) => (*fn_ as *const ()).cast(),
        _ => ptr::null(),
    })
    .unwrap_or(ptr::null())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_pcallk(
    state: *mut lua_State,
    nargs: c_int,
    nresults: c_int,
    _msgh: c_int,
    _ctx: lua_KContext,
    _k: lua_KFunction,
) -> c_int {
    let prepared = match prepare_c_call(state, nargs, nresults) {
        Ok(prepared) => prepared,
        Err(message) => {
            push_call_error(state, &message);
            return LUA_ERRRUN;
        }
    };

    let call_result = panic::catch_unwind(AssertUnwindSafe(|| {
        let Some(function) = prepared.function else {
            return Err("attempt to call a null C function".to_owned());
        };
        // SAFETY: The C function pointer came from `lua_pushcclosure`. The
        // protected boundary catches Rust unwinds from `extern "C-unwind"`
        // callbacks and restores the stack below.
        Ok(unsafe { function(state) })
    }));

    match call_result {
        Ok(Ok(return_count)) => finish_c_call(state, prepared, return_count, nresults),
        Ok(Err(message)) => finish_c_call_error(state, prepared, &message),
        Err(payload) => finish_c_call_error(state, prepared, &panic_message(payload.as_ref())),
    }
}

/// Packaged C API header directory exposed by the build script.
pub const INCLUDE_DIR: &str = env!("ELARA_CAPI_INCLUDE_DIR");

/// Current-version `lua.h` header contents packaged with this crate.
pub const LUA_H: &str = include_str!("../include/lua.h");

/// Current-version `lauxlib.h` header contents packaged with this crate.
pub const LAUXLIB_H: &str = include_str!("../include/lauxlib.h");

/// Current-version `lualib.h` header contents packaged with this crate.
pub const LUALIB_H: &str = include_str!("../include/lualib.h");

fn state_mut(state: *mut lua_State) -> Option<&'static mut lua_State> {
    if state.is_null() {
        return None;
    }

    // SAFETY: C API callers must pass a live `lua_State` allocated by
    // `lua_newstate`. The returned borrow is limited to each FFI call.
    Some(unsafe { &mut *state })
}

fn with_state<R>(state: *mut lua_State, f: impl FnOnce(&lua_State) -> R) -> Option<R> {
    state_mut(state).map(|state| f(state))
}

fn with_value<R>(state: *mut lua_State, idx: c_int, f: impl FnOnce(&CValue) -> R) -> Option<R> {
    let state = state_mut(state)?;
    let index = stack_index(state, idx)?;
    Some(f(&state.stack[index]))
}

fn stack_index(state: &lua_State, idx: c_int) -> Option<usize> {
    if idx > 0 {
        let index = current_base(state).checked_add((idx - 1) as usize)?;
        return (index < state.stack.len()).then_some(index);
    }

    if idx < 0 && idx > LUA_REGISTRYINDEX {
        let index = state.stack.len() as isize + idx as isize;
        return (0..state.stack.len() as isize)
            .contains(&index)
            .then_some(index as usize);
    }

    None
}

fn current_base(state: &lua_State) -> usize {
    state.call_bases.last().copied().unwrap_or(0)
}

struct PreparedCall {
    function_index: usize,
    function: lua_CFunction,
}

fn prepare_c_call(
    state: *mut lua_State,
    nargs: c_int,
    nresults: c_int,
) -> Result<PreparedCall, String> {
    if nargs < 0 {
        return Err("negative argument count".to_owned());
    }
    if nresults < LUA_MULTRET {
        return Err("invalid result count".to_owned());
    }

    let Some(state) = state_mut(state) else {
        return Err("null lua_State".to_owned());
    };
    let nargs = nargs as usize;
    let base = current_base(state);
    let visible_top = state.stack.len().saturating_sub(base);
    if visible_top < nargs.saturating_add(1) {
        return Err("not enough values for C call".to_owned());
    }

    let function_index = state.stack.len() - nargs - 1;
    let function = match state.stack.get(function_index) {
        Some(CValue::CFunction(function)) => *function,
        Some(_) => return Err("attempt to call a non-function value".to_owned()),
        None => return Err("missing C function value".to_owned()),
    };
    state.stack.remove(function_index);
    if state.call_bases.try_reserve(1).is_err() {
        state
            .stack
            .insert(function_index, CValue::CFunction(function));
        return Err("unable to allocate C call frame".to_owned());
    }
    state.call_bases.push(function_index);

    Ok(PreparedCall {
        function_index,
        function,
    })
}

fn finish_c_call(
    state: *mut lua_State,
    prepared: PreparedCall,
    return_count: c_int,
    wanted_count: c_int,
) -> c_int {
    if return_count < 0 {
        return finish_c_call_error(
            state,
            prepared,
            "C function returned a negative result count",
        );
    }

    let Some(state) = state_mut(state) else {
        return LUA_ERRRUN;
    };
    pop_call_base(state, prepared.function_index);

    let return_count = return_count as usize;
    let available = state.stack.len().saturating_sub(prepared.function_index);
    if return_count > available {
        let message = "C function returned more results than are on the stack";
        return push_prepared_error(state, prepared.function_index, message);
    }

    let result_start = state.stack.len() - return_count;
    let mut results = state.stack[result_start..].to_vec();
    state.stack.truncate(prepared.function_index);

    if wanted_count != LUA_MULTRET {
        normalize_fixed_results(&mut results, wanted_count as usize);
    }

    if state.stack.try_reserve(results.len()).is_err() {
        return push_prepared_error(
            state,
            prepared.function_index,
            "unable to allocate C call results",
        );
    }
    state.stack.extend(results);
    LUA_OK
}

fn finish_c_call_error(state: *mut lua_State, prepared: PreparedCall, message: &str) -> c_int {
    let Some(state) = state_mut(state) else {
        return LUA_ERRRUN;
    };
    pop_call_base(state, prepared.function_index);
    push_prepared_error(state, prepared.function_index, message)
}

fn push_call_error(state: *mut lua_State, message: &str) {
    if let Some(state) = state_mut(state) {
        let base = current_base(state);
        push_prepared_error(state, base, message);
    }
}

fn push_prepared_error(state: &mut lua_State, stack_len: usize, message: &str) -> c_int {
    state.stack.truncate(stack_len);
    push_string_payload(state, message.as_bytes());
    LUA_ERRRUN
}

fn pop_call_base(state: &mut lua_State, expected_base: usize) {
    if state.call_bases.last().copied() == Some(expected_base) {
        state.call_bases.pop();
    }
}

fn normalize_fixed_results(results: &mut Vec<CValue>, wanted_count: usize) {
    if results.len() > wanted_count {
        results.drain(..results.len() - wanted_count);
    }
    results.resize(wanted_count, CValue::Nil);
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return format!("panic in C function: {message}");
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return format!("panic in C function: {message}");
    }
    "panic in C function".to_owned()
}

fn push_value(state: *mut lua_State, value: CValue) {
    if let Some(state) = state_mut(state) {
        push_stack(state, value);
    }
}

fn push_stack(state: &mut lua_State, value: CValue) -> bool {
    if state.stack.try_reserve(1).is_err() {
        return false;
    }
    state.stack.push(value);
    true
}

fn resize_stack(state: &mut lua_State, len: usize) -> bool {
    if len > state.stack.len() && state.stack.try_reserve(len - state.stack.len()).is_err() {
        return false;
    }
    state.stack.resize(len, CValue::Nil);
    true
}

fn push_string_payload(state: &mut lua_State, payload: &[u8]) -> *const c_char {
    let Some(required_len) = payload.len().checked_add(1) else {
        return ptr::null();
    };
    let mut bytes = Vec::new();
    if bytes.try_reserve(required_len).is_err() {
        return ptr::null();
    }
    bytes.extend_from_slice(payload);
    bytes.push(0);
    let ptr = bytes.as_ptr().cast();
    if push_stack(state, CValue::String(bytes)) {
        ptr
    } else {
        ptr::null()
    }
}

fn set_out_bool(out: *mut c_int, value: bool) {
    if !out.is_null() {
        // SAFETY: The caller supplied a writable out pointer or null.
        unsafe { *out = c_bool(value) };
    }
}

fn set_out_len(out: *mut usize, value: usize) {
    if !out.is_null() {
        // SAFETY: The caller supplied a writable out pointer or null.
        unsafe { *out = value };
    }
}

fn c_bool(value: bool) -> c_int {
    c_int::from(value)
}

fn usize_to_c_int(value: usize) -> c_int {
    c_int::try_from(value).unwrap_or(c_int::MAX)
}

fn string_payload(bytes: &[u8]) -> Option<&[u8]> {
    bytes.strip_suffix(&[0])
}

#[cfg(test)]
mod tests;
