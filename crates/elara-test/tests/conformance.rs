use std::{fs, path::PathBuf};

use elara_api::Lua;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedClass {
    Success,
    Error,
}

#[test]
fn conformance_language_fixtures() {
    assert_fixture_class("language/return_42.lua", ExpectedClass::Success);
}

#[test]
fn conformance_standard_library_fixtures() {
    assert_fixture_class("stdlib/math_abs.lua", ExpectedClass::Success);
}

#[test]
fn conformance_error_fixtures() {
    assert_fixture_class("errors/non_callable.lua", ExpectedClass::Error);
}

#[test]
fn conformance_coroutine_fixtures() {
    assert_fixture_class("coroutine/wrap.lua", ExpectedClass::Success);
}

fn assert_fixture_class(path: &str, expected: ExpectedClass) {
    let source = fs::read_to_string(fixture_path(path)).expect("fixture should be readable");
    let actual = match Lua::new().eval(source) {
        Ok(_) => ExpectedClass::Success,
        Err(_) => ExpectedClass::Error,
    };
    assert_eq!(actual, expected, "fixture class mismatch for {path}");
}

fn fixture_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance")
        .join(path)
}
