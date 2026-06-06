//! Current Lua language target for Elara.
//!
//! Elara tracks one current stable Lua target on main. Historical Lua versions
//! should live in tags or maintenance branches, not in runtime dialect switches.

use core::fmt;

/// Semantic Lua release number targeted by Elara.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LuaVersion {
    major: u8,
    minor: u8,
    patch: u8,
}

impl LuaVersion {
    /// Creates a Lua version number.
    #[must_use]
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Major version component.
    #[must_use]
    pub const fn major(self) -> u8 {
        self.major
    }

    /// Minor version component.
    #[must_use]
    pub const fn minor(self) -> u8 {
        self.minor
    }

    /// Patch release component.
    #[must_use]
    pub const fn patch(self) -> u8 {
        self.patch
    }
}

impl fmt::Display for LuaVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Current Lua specification metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LuaSpec {
    /// Stable language series name.
    pub version_name: &'static str,
    /// Exact stable release name.
    pub release_name: &'static str,
    /// Exact stable release number.
    pub version: LuaVersion,
    /// Reference manual for the current target.
    pub manual_url: &'static str,
}

/// Exact current Lua release targeted by Elara.
pub const LUA_VERSION: LuaVersion = LuaVersion::new(5, 5, 0);

/// Current Lua specification targeted by Elara.
pub const LUA_SPEC: LuaSpec = LuaSpec {
    version_name: "Lua 5.5",
    release_name: "Lua 5.5.0",
    version: LUA_VERSION,
    manual_url: "https://www.lua.org/manual/5.5/manual.html",
};

#[cfg(test)]
mod tests {
    use super::{LUA_SPEC, LUA_VERSION, LuaVersion};

    #[test]
    fn current_lua_version_is_5_5_0() {
        assert_eq!(LUA_VERSION, LuaVersion::new(5, 5, 0));
        assert_eq!(LUA_VERSION.major(), 5);
        assert_eq!(LUA_VERSION.minor(), 5);
        assert_eq!(LUA_VERSION.patch(), 0);
    }

    #[test]
    fn current_lua_spec_names_single_release_target() {
        assert_eq!(LUA_SPEC.version_name, "Lua 5.5");
        assert_eq!(LUA_SPEC.release_name, "Lua 5.5.0");
        assert_eq!(LUA_SPEC.version, LUA_VERSION);
        assert_eq!(
            LUA_SPEC.manual_url,
            "https://www.lua.org/manual/5.5/manual.html"
        );
    }

    #[test]
    fn lua_version_displays_release_number() {
        assert_eq!(LUA_VERSION.to_string(), "5.5.0");
    }
}
