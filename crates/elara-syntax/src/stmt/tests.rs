use elara_core::SourceId;

use crate::{FunctionScope, GlobalDecl, Param, StmtKind, parse_chunk};

fn parse(source: &str) -> crate::Block<'_> {
    let parsed = parse_chunk(SourceId::new(0), source);
    assert_eq!(parsed.diagnostics, Vec::new());
    parsed.block
}

#[test]
fn stmt_parses_assignment_and_call() {
    let block = parse("a, b = 1, f()\nf(2)");

    assert_eq!(block.statements().len(), 2);
    match block.statements()[0].kind() {
        StmtKind::Assign { targets, values } => {
            assert_eq!(targets.len(), 2);
            assert_eq!(values.len(), 2);
        }
        kind => panic!("expected assignment, got {kind:?}"),
    }
    assert!(matches!(block.statements()[1].kind(), StmtKind::Call(_)));
}

#[test]
fn stmt_parses_local_and_global_declarations() {
    let block = parse("local<const> x = 1\nglobal y = 2\nglobal<const> *");

    assert_eq!(block.statements().len(), 3);
    match block.statements()[0].kind() {
        StmtKind::Local { names, values } => {
            assert_eq!(names[0].name, "x");
            assert_eq!(names[0].attribute, Some("const"));
            assert_eq!(values.len(), 1);
        }
        kind => panic!("expected local declaration, got {kind:?}"),
    }
    assert!(matches!(
        block.statements()[1].kind(),
        StmtKind::Global(GlobalDecl::Names { .. })
    ));
    assert!(matches!(
        block.statements()[2].kind(),
        StmtKind::Global(GlobalDecl::All {
            attribute: Some("const")
        })
    ));
}

#[test]
fn stmt_parses_function_declarations() {
    let block = parse("function t.a:f(x, ... rest) return x end\nlocal function g() end");

    assert_eq!(block.statements().len(), 2);
    match block.statements()[0].kind() {
        StmtKind::Function { scope, name, body } => {
            assert_eq!(*scope, FunctionScope::Plain);
            assert_eq!(name.base, "t");
            assert_eq!(name.fields, vec!["a"]);
            assert_eq!(name.method, Some("f"));
            assert_eq!(
                body.params,
                vec![Param::Name("x"), Param::Vararg(Some("rest"))]
            );
            assert_eq!(body.block.statements().len(), 1);
        }
        kind => panic!("expected function declaration, got {kind:?}"),
    }
    assert!(matches!(
        block.statements()[1].kind(),
        StmtKind::Function {
            scope: FunctionScope::Local,
            ..
        }
    ));
}

#[test]
fn stmt_parses_control_flow() {
    let block = parse(
        "if a then b = 1 elseif c then b = 2 else b = 3 end\nwhile a do break end\nrepeat a = a - 1 until a",
    );

    assert_eq!(block.statements().len(), 3);
    match block.statements()[0].kind() {
        StmtKind::If {
            clauses,
            else_block,
        } => {
            assert_eq!(clauses.len(), 2);
            assert!(else_block.is_some());
        }
        kind => panic!("expected if statement, got {kind:?}"),
    }
    assert!(matches!(
        block.statements()[1].kind(),
        StmtKind::While { .. }
    ));
    assert!(matches!(
        block.statements()[2].kind(),
        StmtKind::Repeat { .. }
    ));
}

#[test]
fn stmt_parses_for_return_goto_and_labels() {
    let block = parse(
        "::again::\nfor i = 1, 10, 2 do goto again end\nfor k, v in pairs(t) do return k, v end",
    );

    assert_eq!(block.statements().len(), 3);
    assert!(matches!(
        block.statements()[0].kind(),
        StmtKind::Label("again")
    ));
    assert!(matches!(
        block.statements()[1].kind(),
        StmtKind::NumericFor { name: "i", .. }
    ));
    match block.statements()[2].kind() {
        StmtKind::GenericFor { names, body, .. } => {
            assert_eq!(names, &vec!["k", "v"]);
            assert!(matches!(
                body.statements()[0].kind(),
                StmtKind::Return(values) if values.len() == 2
            ));
        }
        kind => panic!("expected generic for, got {kind:?}"),
    }
}

#[test]
fn stmt_reports_missing_assignment_operator() {
    let parsed = parse_chunk(SourceId::new(0), "a 1");

    assert!(!parsed.diagnostics.is_empty());
    assert_eq!(
        parsed.diagnostics[0].message(),
        "expected '=' after assignment targets"
    );
}
