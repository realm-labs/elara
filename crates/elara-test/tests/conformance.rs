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
}

#[test]
fn conformance_standard_library_fixtures() {
    assert_success_fixture("stdlib/math_abs.lua", vec![Value::integer(42)]);
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
        "stdlib/os_package_debug.lua",
        vec![
            Value::float(6.0),
            Value::integer(115),
            Value::integer(116),
            Value::integer(115),
        ],
    );
}

#[test]
fn conformance_error_fixtures() {
    assert_error_fixture("errors/non_callable.lua");
    assert_error_fixture("errors/base_error.lua");
}

#[test]
fn conformance_coroutine_fixtures() {
    assert_success_fixture("coroutine/wrap.lua", vec![Value::integer(42)]);
    assert_success_fixture("coroutine/resume_status.lua", vec![Value::boolean(true)]);
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
