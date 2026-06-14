//! Optional Lua 5.5 C API layer for Elara.
//!
//! This crate is the optional source-compatible C API surface for the current
//! Lua target only. It should be implemented on top of the safe Rust API where
//! practical.
//!
//! It must not introduce compatibility branches for old Lua C APIs.

/// Packaged C API header directory exposed by the build script.
pub const INCLUDE_DIR: &str = env!("ELARA_CAPI_INCLUDE_DIR");

/// Current-version `lua.h` header contents packaged with this crate.
pub const LUA_H: &str = include_str!("../include/lua.h");

/// Current-version `lauxlib.h` header contents packaged with this crate.
pub const LAUXLIB_H: &str = include_str!("../include/lauxlib.h");

/// Current-version `lualib.h` header contents packaged with this crate.
pub const LUALIB_H: &str = include_str!("../include/lualib.h");

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{INCLUDE_DIR, LAUXLIB_H, LUA_H, LUALIB_H};

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
}
