use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn c_integration_module_compiles_against_packaged_headers() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let include_dir = format!("{manifest_dir}/include");
    let work_dir = std::env::temp_dir().join(format!(
        "elara-capi-c-integration-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after unix epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&work_dir).expect("test work directory should be created");

    let source = work_dir.join("elara_capi_probe.c");
    let object = work_dir.join("elara_capi_probe.o");
    fs::write(&source, C_MODULE).expect("C probe source should be written");

    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    let output = Command::new(&compiler)
        .arg("-std=c99")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-I")
        .arg(&include_dir)
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .output()
        .unwrap_or_else(|error| panic!("failed to run C compiler `{compiler}`: {error}"));

    assert!(
        output.status.success(),
        "C probe should compile against packaged Elara headers\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(object.is_file());

    let _ = fs::remove_dir_all(&work_dir);
}

const C_MODULE: &str = r#"
#include "lua.h"
#include "lauxlib.h"
#include "lualib.h"

static int elara_probe_add(lua_State *L) {
    int lhs_ok = 0;
    int rhs_ok = 0;
    lua_Integer lhs = lua_tointegerx(L, 1, &lhs_ok);
    lua_Integer rhs = lua_tointegerx(L, 2, &rhs_ok);

    if (!lhs_ok || !rhs_ok) {
        lua_pushstring(L, "expected integer arguments");
        return 1;
    }

    lua_pushinteger(L, lhs + rhs);
    return 1;
}

int luaopen_elara_capi_probe(lua_State *L) {
    lua_pushcfunction(L, elara_probe_add);
    lua_pushinteger(L, 20);
    lua_pushinteger(L, 22);

    if (lua_pcall(L, 2, 1, 0) != LUA_OK) {
        return 1;
    }

    if (!lua_isinteger(L, -1) || lua_tointeger(L, -1) != 42) {
        lua_pushstring(L, "unexpected C API arithmetic result");
        return 1;
    }

    lua_pushstring(L, LUA_VERSION);
    return 2;
}
"#;
