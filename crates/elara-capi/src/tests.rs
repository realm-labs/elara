use std::ffi::{CStr, CString, c_int, c_void};
use std::path::Path;

use super::{
    INCLUDE_DIR, LAUXLIB_H, LUA_ERRRUN, LUA_H, LUA_OK, LUA_TBOOLEAN, LUA_TFUNCTION, LUA_TNIL,
    LUA_TNONE, LUA_TNUMBER, LUA_TSTRING, LUALIB_H, lua_Alloc, lua_State, lua_checkstack, lua_close,
    lua_copy, lua_gettop, lua_iscfunction, lua_isinteger, lua_isnumber, lua_isstring, lua_newstate,
    lua_pcallk, lua_pushboolean, lua_pushcclosure, lua_pushinteger, lua_pushlstring, lua_pushnil,
    lua_pushnumber, lua_pushstring, lua_pushvalue, lua_rotate, lua_settop, lua_toboolean,
    lua_tocfunction, lua_tointegerx, lua_tolstring, lua_tonumberx, lua_type, lua_typename,
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

unsafe extern "C-unwind" fn c_function_add(state: *mut lua_State) -> c_int {
    // SAFETY: The callback is invoked by the protected C API test with two
    // integer arguments on the active C call frame.
    unsafe {
        assert_eq!(lua_gettop(state), 2);
        let lhs = lua_tointegerx(state, 1, std::ptr::null_mut());
        let rhs = lua_tointegerx(state, 2, std::ptr::null_mut());
        lua_pushinteger(state, lhs + rhs);
    }
    1
}

unsafe extern "C-unwind" fn c_function_returns_pair(state: *mut lua_State) -> c_int {
    // SAFETY: The callback is invoked with a live state by `lua_pcallk`.
    unsafe {
        lua_pushinteger(state, 10);
        lua_pushinteger(state, 20);
    }
    2
}

unsafe extern "C-unwind" fn c_function_panics(_state: *mut lua_State) -> c_int {
    panic!("contained callback panic");
}

#[test]
fn c_function_pushcclosure_round_trips_function_pointer() {
    let state = TestState::new();

    // SAFETY: The test pushes and reads a C function value on a live state.
    unsafe {
        lua_pushcclosure(state.as_ptr(), Some(c_function_add), 0);
        assert_eq!(lua_type(state.as_ptr(), -1), LUA_TFUNCTION);
        assert_eq!(lua_iscfunction(state.as_ptr(), -1), 1);
        assert_eq!(
            lua_tocfunction(state.as_ptr(), -1).map(|function| function as usize),
            Some(c_function_add as *const () as usize)
        );
    }
}

#[test]
fn c_function_pcall_invokes_callback_and_replaces_args_with_results() {
    let state = TestState::new();

    // SAFETY: The test calls a registered C function with two integer args.
    unsafe {
        lua_pushcclosure(state.as_ptr(), Some(c_function_add), 0);
        lua_pushinteger(state.as_ptr(), 2);
        lua_pushinteger(state.as_ptr(), 3);

        assert_eq!(lua_pcallk(state.as_ptr(), 2, 1, 0, 0, None), LUA_OK);
        assert_eq!(lua_gettop(state.as_ptr()), 1);
        assert_eq!(lua_tointegerx(state.as_ptr(), -1, std::ptr::null_mut()), 5);
    }
}

#[test]
fn c_function_pcall_normalizes_fixed_and_multret_results() {
    let state = TestState::new();

    // SAFETY: The test calls registered C functions on a live state.
    unsafe {
        lua_pushcclosure(state.as_ptr(), Some(c_function_returns_pair), 0);
        assert_eq!(lua_pcallk(state.as_ptr(), 0, 1, 0, 0, None), LUA_OK);
        assert_eq!(lua_gettop(state.as_ptr()), 1);
        assert_eq!(lua_tointegerx(state.as_ptr(), -1, std::ptr::null_mut()), 20);

        lua_settop(state.as_ptr(), 0);
        lua_pushcclosure(state.as_ptr(), Some(c_function_returns_pair), 0);
        assert_eq!(lua_pcallk(state.as_ptr(), 0, 3, 0, 0, None), LUA_OK);
        assert_eq!(lua_gettop(state.as_ptr()), 3);
        assert_eq!(lua_type(state.as_ptr(), -1), LUA_TNIL);
        assert_eq!(lua_tointegerx(state.as_ptr(), 1, std::ptr::null_mut()), 10);
        assert_eq!(lua_tointegerx(state.as_ptr(), 2, std::ptr::null_mut()), 20);
    }
}

#[test]
fn c_function_pcall_contains_callback_panic_as_runtime_error() {
    let state = TestState::new();

    // SAFETY: The panic crosses an `extern "C-unwind"` callback boundary
    // and is caught by `lua_pcallk`.
    unsafe {
        lua_pushcclosure(state.as_ptr(), Some(c_function_panics), 0);
        assert_eq!(lua_pcallk(state.as_ptr(), 0, 0, 0, 0, None), LUA_ERRRUN);
        assert_eq!(lua_gettop(state.as_ptr()), 1);

        let mut len = 0;
        let error = lua_tolstring(state.as_ptr(), -1, &mut len);
        let message = std::slice::from_raw_parts(error.cast::<u8>(), len);
        assert!(
            std::str::from_utf8(message)
                .unwrap()
                .contains("contained callback panic")
        );
    }
}
