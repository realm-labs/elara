use std::{fs, io, path::PathBuf};

use elara_test::{DifferentialRunner, LuaRunner};

const DIFFERENTIAL_FIXTURES: &[&str] = &[
    "language/return_42.lua",
    "language/control_flow.lua",
    "language/bitwise.lua",
    "language/varargs.lua",
    "language/table_fields.lua",
    "language/closures.lua",
    "language/global_declarations.lua",
    "language/metamethods.lua",
    "stdlib/math_abs.lua",
    "stdlib/base_table.lua",
    "stdlib/base_rawlen.lua",
    "stdlib/base_raw_access.lua",
    "stdlib/base_iteration.lua",
    "stdlib/base_conversion.lua",
    "stdlib/base_pcall.lua",
    "stdlib/base_xpcall.lua",
    "stdlib/math_string_patterns.lua",
    "stdlib/string_format.lua",
    "stdlib/string_ops.lua",
    "stdlib/string_byte_char.lua",
    "stdlib/math_numeric.lua",
    "stdlib/math_random.lua",
    "stdlib/table_string_utf8.lua",
    "stdlib/table_mutation.lua",
    "stdlib/table_sort.lua",
    "stdlib/io_stubs.lua",
    "stdlib/io_type.lua",
    "stdlib/utf8_iteration.lua",
    "stdlib/utf8_char.lua",
    "stdlib/os_package_debug.lua",
    "stdlib/package_require.lua",
    "stdlib/package_searchpath.lua",
    "stdlib/package_config.lua",
    "stdlib/package_loadlib.lua",
    "stdlib/debug_introspection.lua",
    "stdlib/debug_registry.lua",
    "stdlib/debug_upvalues.lua",
    "stdlib/debug_traceback.lua",
    "stdlib/debug_metatable.lua",
    "stdlib/debug_hooks.lua",
    "stdlib/os_time_date.lua",
    "stdlib/os_locale.lua",
    "stdlib/os_execute.lua",
    "stdlib/os_clock.lua",
    "stdlib/os_tmpname.lua",
    "stdlib/os_getenv.lua",
    "stdlib/os_remove.lua",
    "stdlib/os_rename.lua",
    "coroutine/wrap.lua",
    "coroutine/resume_status.lua",
    "coroutine/lifecycle.lua",
    "coroutine/close.lua",
    "errors/non_callable.lua",
    "errors/base_error.lua",
    "errors/syntax_unclosed.lua",
    "errors/bad_argument.lua",
    "errors/non_table_index.lua",
    "errors/arithmetic_type.lua",
    "errors/debug_uservalue.lua",
];

#[test]
fn differential_fixtures_match_official_lua_error_classes_when_configured() {
    let Some(official) = LuaRunner::from_env() else {
        eprintln!("skipping differential fixtures; set ELARA_LUA to an official Lua executable");
        return;
    };
    let runner = DifferentialRunner::new(official);

    for fixture in DIFFERENTIAL_FIXTURES {
        assert_fixture_class(&runner, fixture).expect("differential fixture should run");
    }
}

fn assert_fixture_class(runner: &DifferentialRunner, path: &str) -> io::Result<()> {
    let source = fs::read_to_string(fixture_path(path))?;
    let comparison = runner.compare_source(&source)?;
    assert!(
        comparison.same_error_class(),
        "fixture {path} differed: official={:?} elara={:?}",
        comparison.official.class(),
        comparison.elara.class()
    );
    Ok(())
}

fn fixture_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance")
        .join(path)
}
