use std::{fs, path::PathBuf};

use elara_api::Lua;
use elara_core::Value;

#[test]
fn conformance_language_fixtures() {
    assert_success_fixture("language/return_42.lua", vec![Value::integer(42)]);
    assert_success_fixture("language/control_flow.lua", vec![Value::integer(17)]);
    assert_success_fixture(
        "language/bitwise.lua",
        vec![
            Value::integer(8),
            Value::integer(14),
            Value::integer(6),
            Value::integer(16),
            Value::integer(4),
            Value::integer(0),
            Value::integer(-1),
        ],
    );
    assert_success_fixture("language/varargs.lua", vec![Value::integer(10)]);
    assert_success_fixture(
        "language/table_fields.lua",
        vec![
            Value::integer(10),
            Value::integer(20),
            Value::integer(30),
            Value::integer(40),
            Value::integer(40),
            Value::integer(4),
        ],
    );
    assert_success_fixture(
        "language/closures.lua",
        vec![Value::integer(42), Value::integer(42)],
    );
    assert_success_fixture(
        "language/global_declarations.lua",
        vec![Value::integer(42), Value::integer(42)],
    );
    assert_success_fixture("language/metamethods.lua", vec![Value::integer(42)]);
}

#[test]
fn conformance_standard_library_fixtures() {
    assert_success_fixture("stdlib/math_abs.lua", vec![Value::integer(42)]);
    assert_success_fixture(
        "stdlib/base_table.lua",
        vec![
            Value::integer(110),
            Value::boolean(true),
            Value::boolean(true),
            Value::integer(3),
            Value::integer(2),
            Value::integer(100),
            Value::integer(101),
        ],
    );
    assert_success_fixture(
        "stdlib/base_rawlen.lua",
        vec![Value::integer(3), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/base_raw_access.lua",
        vec![Value::integer(42), Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/base_conversion.lua",
        vec![
            Value::integer(20),
            Value::integer(3),
            Value::integer(10),
            Value::boolean(true),
            Value::integer(116),
        ],
    );
    assert_success_fixture(
        "stdlib/base_pcall.lua",
        vec![Value::boolean(true), Value::integer(42)],
    );
    assert_success_fixture(
        "stdlib/base_xpcall.lua",
        vec![Value::boolean(false), Value::integer(9)],
    );
    assert_success_fixture(
        "stdlib/math_string_patterns.lua",
        vec![
            Value::integer(7),
            Value::integer(2),
            Value::integer(2),
            Value::integer(42),
            Value::integer(105),
            Value::integer(4),
            Value::integer(3),
            Value::integer(3),
        ],
    );
    assert_success_fixture(
        "stdlib/string_format.lua",
        vec![
            Value::integer(7),
            Value::integer(48),
            Value::integer(102),
            Value::integer(58),
            Value::integer(43),
        ],
    );
    assert_success_fixture(
        "stdlib/string_ops.lua",
        vec![
            Value::integer(8),
            Value::integer(4),
            Value::integer(99),
            Value::integer(90),
            Value::integer(97),
        ],
    );
    assert_success_fixture(
        "stdlib/math_numeric.lua",
        vec![
            Value::integer(3),
            Value::integer(4),
            Value::float(9.0),
            Value::float(3.0),
            Value::float(8.0),
            Value::integer(12),
            Value::integer(102),
            Value::boolean(false),
        ],
    );
    assert_success_fixture(
        "stdlib/math_random.lua",
        vec![Value::integer(1), Value::integer(7), Value::integer(9)],
    );
    assert_success_fixture(
        "stdlib/table_string_utf8.lua",
        vec![
            Value::integer(1),
            Value::integer(2),
            Value::integer(3),
            Value::integer(4),
            Value::integer(3),
            Value::integer(2),
            Value::integer(66),
        ],
    );
    assert_success_fixture(
        "stdlib/table_mutation.lua",
        vec![
            Value::integer(5),
            Value::integer(97),
            Value::integer(98),
            Value::integer(20),
            Value::integer(30),
            Value::integer(40),
            Value::integer(40),
            Value::integer(8),
        ],
    );
    assert_success_fixture(
        "stdlib/table_sort.lua",
        vec![Value::integer(1), Value::integer(2), Value::integer(3)],
    );
    assert_success_fixture(
        "stdlib/io_stubs.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/io_type.lua",
        vec![Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/utf8_iteration.lua",
        vec![
            Value::integer(198),
            Value::integer(3),
            Value::integer(2),
            Value::integer(3),
            Value::integer(91),
        ],
    );
    assert_success_fixture(
        "stdlib/utf8_char.lua",
        vec![Value::integer(1), Value::integer(65)],
    );
    assert_success_fixture(
        "stdlib/os_package_debug.lua",
        vec![
            Value::float(6.0),
            Value::integer(115),
            Value::integer(116),
            Value::integer(115),
        ],
    );
    assert_success_fixture(
        "stdlib/package_require.lua",
        vec![
            Value::integer(77),
            Value::integer(77),
            Value::integer(77),
            Value::integer(23),
        ],
    );
    assert_success_fixture(
        "stdlib/package_searchpath.lua",
        vec![Value::boolean(true), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/package_config.lua",
        vec![Value::integer(115), Value::integer(10)],
    );
    assert_success_fixture(
        "stdlib/package_loadlib.lua",
        vec![Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/debug_introspection.lua",
        vec![
            Value::integer(76),
            Value::integer(0),
            Value::boolean(false),
            Value::integer(109),
            Value::integer(0),
            Value::boolean(false),
            Value::boolean(false),
            Value::integer(0),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_registry.lua",
        vec![Value::integer(42), Value::boolean(true)],
    );
    assert_success_fixture(
        "stdlib/debug_upvalues.lua",
        vec![
            Value::integer(10),
            Value::integer(30),
            Value::integer(40),
            Value::integer(40),
            Value::boolean(false),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_traceback.lua",
        vec![Value::integer(98), Value::integer(115)],
    );
    assert_success_fixture(
        "stdlib/debug_metatable.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/debug_hooks.lua",
        vec![
            Value::boolean(true),
            Value::boolean(true),
            Value::boolean(true),
        ],
    );
    assert_success_fixture(
        "stdlib/os_time_date.lua",
        vec![
            Value::integer(1970),
            Value::integer(1),
            Value::integer(1),
            Value::integer(0),
            Value::integer(0),
            Value::integer(0),
            Value::float(86400.0),
            Value::integer(115),
        ],
    );
    assert_success_fixture(
        "stdlib/os_locale.lua",
        vec![Value::integer(67), Value::integer(67), Value::boolean(true)],
    );
    assert_success_fixture("stdlib/os_execute.lua", vec![Value::boolean(true)]);
    assert_success_fixture("stdlib/os_tmpname.lua", vec![Value::integer(115)]);
    assert_success_fixture("stdlib/os_getenv.lua", vec![Value::boolean(true)]);
    assert_success_fixture("stdlib/os_remove.lua", vec![Value::boolean(true)]);
}

#[test]
fn conformance_error_fixtures() {
    assert_error_fixture("errors/non_callable.lua");
    assert_error_fixture("errors/base_error.lua");
    assert_error_fixture("errors/syntax_unclosed.lua");
    assert_error_fixture("errors/bad_argument.lua");
    assert_error_fixture("errors/non_table_index.lua");
    assert_error_fixture("errors/arithmetic_type.lua");
    assert_error_fixture("errors/debug_uservalue.lua");
}

#[test]
fn conformance_coroutine_fixtures() {
    assert_success_fixture("coroutine/wrap.lua", vec![Value::integer(42)]);
    assert_success_fixture("coroutine/resume_status.lua", vec![Value::boolean(true)]);
    assert_success_fixture(
        "coroutine/lifecycle.lua",
        vec![
            Value::integer(115),
            Value::boolean(true),
            Value::boolean(true),
            Value::integer(100),
        ],
    );
    assert_success_fixture("coroutine/close.lua", vec![Value::boolean(true)]);
}

fn assert_success_fixture(path: &str, expected: Vec<Value>) {
    let source = fs::read_to_string(fixture_path(path)).expect("fixture should be readable");
    let actual = Lua::new()
        .eval(source)
        .unwrap_or_else(|error| panic!("fixture {path} should succeed: {error:?}"));
    assert_eq!(actual, expected, "fixture values mismatch for {path}");
}

fn assert_error_fixture(path: &str) {
    let source = fs::read_to_string(fixture_path(path)).expect("fixture should be readable");
    let result = Lua::new().eval(source);
    assert!(result.is_err(), "fixture {path} should error: {result:?}");
}

fn fixture_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance")
        .join(path)
}
