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
//! indices. Protected call panic containment is handled by the later C-call
//! trampoline layer.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;

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
pub type lua_CFunction = Option<unsafe extern "C" fn(*mut lua_State) -> c_int>;
pub type lua_Alloc =
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void>;

/// Opaque current-version Lua C API state.
#[repr(C)]
pub struct lua_State {
    stack: Vec<CValue>,
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
    with_state(state, |state| usize_to_c_int(state.stack.len())).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lua_settop(state: *mut lua_State, idx: c_int) {
    let Some(state) = state_mut(state) else {
        return;
    };

    if idx >= 0 {
        resize_stack(state, idx as usize);
        return;
    }

    let next_top = state.stack.len() as isize + idx as isize + 1;
    state.stack.truncate(next_top.max(0) as usize);
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
        let index = (idx - 1) as usize;
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
mod tests {
    use std::ffi::{CStr, CString, c_void};
    use std::path::Path;

    use super::{
        INCLUDE_DIR, LAUXLIB_H, LUA_H, LUA_TBOOLEAN, LUA_TNIL, LUA_TNONE, LUA_TNUMBER, LUA_TSTRING,
        LUALIB_H, lua_Alloc, lua_State, lua_checkstack, lua_close, lua_copy, lua_gettop,
        lua_isinteger, lua_isnumber, lua_isstring, lua_newstate, lua_pushboolean, lua_pushinteger,
        lua_pushlstring, lua_pushnil, lua_pushnumber, lua_pushstring, lua_pushvalue, lua_rotate,
        lua_settop, lua_toboolean, lua_tointegerx, lua_tolstring, lua_tonumberx, lua_type,
        lua_typename,
    };

    unsafe extern "C" fn test_alloc(
        _ud: *mut c_void,
        _ptr: *mut c_void,
        _osize: usize,
        _nsize: usize,
    ) -> *mut c_void {
        std::ptr::null_mut()
    }

    struct TestState(*mut lua_State);

    impl TestState {
        fn new() -> Self {
            // SAFETY: Tests close the state through the RAII guard.
            let alloc: lua_Alloc = Some(test_alloc);
            let state = unsafe { lua_newstate(alloc, std::ptr::null_mut()) };
            assert!(!state.is_null());
            Self(state)
        }

        fn as_ptr(&self) -> *mut lua_State {
            self.0
        }
    }

    impl Drop for TestState {
        fn drop(&mut self) {
            // SAFETY: The guard owns the state returned by `lua_newstate`.
            unsafe { lua_close(self.0) };
        }
    }

    #[test]
    fn c_api_headers_are_packaged() {
        let include_dir = Path::new(INCLUDE_DIR);

        assert!(include_dir.join("lua.h").is_file());
        assert!(include_dir.join("lauxlib.h").is_file());
        assert!(include_dir.join("lualib.h").is_file());
    }

    #[test]
    fn lua_header_targets_current_lua_version_only() {
        assert!(LUA_H.contains("#define LUA_VERSION_MAJOR \"5\""));
        assert!(LUA_H.contains("#define LUA_VERSION_MINOR \"5\""));
        assert!(LUA_H.contains("#define LUA_VERSION_RELEASE \"0\""));
        assert!(LUA_H.contains("#define LUA_VERSION_NUM 505"));
        assert!(!LUA_H.contains("LUA_VERSION_NUM 504"));
    }

    #[test]
    fn auxiliary_headers_include_lua_header() {
        assert!(LAUXLIB_H.contains("#include \"lua.h\""));
        assert!(LUALIB_H.contains("#include \"lua.h\""));
        assert!(LAUXLIB_H.contains("luaL_newstate"));
        assert!(LUALIB_H.contains("luaopen_package"));
    }

    #[test]
    fn stack_newstate_starts_empty_and_settop_grows_with_nil() {
        let state = TestState::new();

        // SAFETY: The test uses a live state and valid C API calls.
        unsafe {
            assert_eq!(lua_gettop(state.as_ptr()), 0);
            lua_settop(state.as_ptr(), 3);
            assert_eq!(lua_gettop(state.as_ptr()), 3);
            assert_eq!(lua_type(state.as_ptr(), 1), LUA_TNIL);
            assert_eq!(lua_type(state.as_ptr(), -1), LUA_TNIL);

            lua_settop(state.as_ptr(), -2);
            assert_eq!(lua_gettop(state.as_ptr()), 2);
            assert_eq!(lua_checkstack(state.as_ptr(), 16), 1);
        }
    }

    #[test]
    fn stack_push_and_type_inspection_round_trips_primitives() {
        let state = TestState::new();

        // SAFETY: The test uses a live state and valid C API calls.
        unsafe {
            lua_pushnil(state.as_ptr());
            lua_pushboolean(state.as_ptr(), 1);
            lua_pushinteger(state.as_ptr(), 42);
            lua_pushnumber(state.as_ptr(), 3.5);

            assert_eq!(lua_gettop(state.as_ptr()), 4);
            assert_eq!(lua_type(state.as_ptr(), 1), LUA_TNIL);
            assert_eq!(lua_type(state.as_ptr(), 2), LUA_TBOOLEAN);
            assert_eq!(lua_type(state.as_ptr(), 3), LUA_TNUMBER);
            assert_eq!(lua_type(state.as_ptr(), 9), LUA_TNONE);
            assert_eq!(lua_isinteger(state.as_ptr(), 3), 1);
            assert_eq!(lua_isinteger(state.as_ptr(), 4), 0);
            assert_eq!(lua_isnumber(state.as_ptr(), 4), 1);
            assert_eq!(lua_isstring(state.as_ptr(), 3), 1);
            assert_eq!(lua_toboolean(state.as_ptr(), 1), 0);
            assert_eq!(lua_toboolean(state.as_ptr(), 2), 1);

            let mut is_num = 0;
            assert_eq!(lua_tointegerx(state.as_ptr(), 3, &mut is_num), 42);
            assert_eq!(is_num, 1);
            assert_eq!(lua_tonumberx(state.as_ptr(), 4, &mut is_num), 3.5);
            assert_eq!(is_num, 1);
            assert_eq!(lua_tonumberx(state.as_ptr(), 1, &mut is_num), 0.0);
            assert_eq!(is_num, 0);
        }
    }

    #[test]
    fn stack_strings_return_stable_bytes() {
        let state = TestState::new();
        let c_string = CString::new("hello").unwrap();

        // SAFETY: The test passes valid string pointers and a live state.
        unsafe {
            let pushed = lua_pushstring(state.as_ptr(), c_string.as_ptr());
            assert_eq!(CStr::from_ptr(pushed).to_str().unwrap(), "hello");
            assert_eq!(lua_type(state.as_ptr(), -1), LUA_TSTRING);

            let bytes = b"a\0b";
            let pushed = lua_pushlstring(state.as_ptr(), bytes.as_ptr().cast(), bytes.len());
            assert!(!pushed.is_null());

            let mut len = 0;
            let returned = lua_tolstring(state.as_ptr(), -1, &mut len);
            assert_eq!(len, 3);
            assert_eq!(
                std::slice::from_raw_parts(returned.cast::<u8>(), len),
                bytes
            );
        }
    }

    #[test]
    fn stack_negative_indices_copy_rotate_and_pop() {
        let state = TestState::new();

        // SAFETY: The test uses a live state and valid stack indices.
        unsafe {
            lua_pushinteger(state.as_ptr(), 1);
            lua_pushinteger(state.as_ptr(), 2);
            lua_pushinteger(state.as_ptr(), 3);
            lua_copy(state.as_ptr(), -1, 1);

            let mut is_num = 0;
            assert_eq!(lua_tointegerx(state.as_ptr(), 1, &mut is_num), 3);
            assert_eq!(is_num, 1);

            lua_rotate(state.as_ptr(), 1, 1);
            assert_eq!(lua_tointegerx(state.as_ptr(), 1, &mut is_num), 3);
            assert_eq!(lua_tointegerx(state.as_ptr(), 2, &mut is_num), 3);
            assert_eq!(lua_tointegerx(state.as_ptr(), 3, &mut is_num), 2);

            lua_pushvalue(state.as_ptr(), 2);
            assert_eq!(lua_gettop(state.as_ptr()), 4);
            assert_eq!(lua_tointegerx(state.as_ptr(), -1, &mut is_num), 3);

            lua_settop(state.as_ptr(), -2);
            assert_eq!(lua_gettop(state.as_ptr()), 3);
        }
    }

    #[test]
    fn stack_typename_returns_current_lua_names() {
        // SAFETY: `lua_typename` returns pointers to static names.
        unsafe {
            assert_eq!(
                CStr::from_ptr(lua_typename(std::ptr::null_mut(), LUA_TNONE)).to_str(),
                Ok("no value")
            );
            assert_eq!(
                CStr::from_ptr(lua_typename(std::ptr::null_mut(), LUA_TNUMBER)).to_str(),
                Ok("number")
            );
        }
    }
}
