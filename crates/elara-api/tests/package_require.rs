use elara_api::{EvalError, eval_simple_source_with_stdlib};
use elara_core::{SourceId, Value};
use elara_stdlib::{StdLib, StdLibProfile};

#[test]
fn package_require_loads_preloaded_module() {
    let profile = StdLibProfile::Custom([StdLib::Package].into_iter().collect());

    let values = eval_simple_source_with_stdlib(
        SourceId::new(0),
        "local function loader()\n  return 42\nend\npackage.preload.mod = loader\nreturn package.require('mod')",
        &profile,
    )
    .expect("package.require should load from package.preload");

    assert_eq!(values.len(), 1);
    assert_eq!(values[0], Value::integer(42));
}

#[test]
fn global_require_loads_preloaded_module() {
    let profile = StdLibProfile::Custom([StdLib::Package].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function loader()\n  return 42\nend\npackage.preload.mod = loader\nreturn require('mod')",
            &profile,
        ),
        Ok(vec![Value::integer(42)])
    );
}

#[test]
fn global_require_uses_custom_package_searcher() {
    let profile = StdLibProfile::Custom([StdLib::Package].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function loader()\n  return 77\nend\nlocal function searcher()\n  return loader, 13\nend\npackage.searchers[1] = searcher\nreturn require('mod')",
            &profile,
        ),
        Ok(vec![Value::integer(77)])
    );
}

#[test]
fn package_require_caches_preload_result() {
    let profile = StdLibProfile::Custom([StdLib::Package].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function loader()\n  return 42\nend\npackage.preload.mod = loader\nlocal first = package.require('mod')\nlocal function other()\n  return 99\nend\npackage.preload.mod = other\nlocal second = package.require('mod')\nreturn first, second, package.loaded.mod",
            &profile,
        ),
        Ok(vec![
            Value::integer(42),
            Value::integer(42),
            Value::integer(42)
        ])
    );
}

#[test]
fn package_require_defaults_nil_loader_result_to_true() {
    let profile = StdLibProfile::Custom([StdLib::Package].into_iter().collect());

    assert_eq!(
        eval_simple_source_with_stdlib(
            SourceId::new(0),
            "local function loader()\n  return nil\nend\npackage.preload.mod = loader\nreturn package.require('mod'), package.loaded.mod",
            &profile,
        ),
        Ok(vec![Value::boolean(true), Value::boolean(true)])
    );
}

#[test]
fn package_require_reports_missing_preload() {
    let profile = StdLibProfile::Custom([StdLib::Package].into_iter().collect());
    let error = eval_simple_source_with_stdlib(
        SourceId::new(0),
        "return package.require('missing')",
        &profile,
    )
    .expect_err("missing preload should raise");

    match error {
        EvalError::Runtime(error) => assert_eq!(
            error.message(),
            "module 'missing' not found:\n\tno field package.preload['missing']"
        ),
        EvalError::Diagnostics(diagnostics) => {
            panic!("expected runtime error, got diagnostics {diagnostics:?}");
        }
    }
}
