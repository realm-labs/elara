use elara_core::SourceId;

use crate::{BinaryOp, Expr, ExprKind, TableFieldKind, TokenKind, UnaryOp, parse_expression};

fn parse(source: &str) -> Expr<'_> {
    let parsed = parse_expression(SourceId::new(0), source);
    assert_eq!(parsed.diagnostics, Vec::new());
    parsed.expression.unwrap()
}

fn binary<'a, 'src>(expr: &'a Expr<'src>) -> (BinaryOp, &'a Expr<'src>, &'a Expr<'src>) {
    match expr.kind() {
        ExprKind::Binary { op, left, right } => (*op, left, right),
        kind => panic!("expected binary expression, got {kind:?}"),
    }
}

#[test]
fn expr_parses_binary_precedence() {
    let expr = parse("a + b * c");
    let (op, left, right) = binary(&expr);

    assert_eq!(op, BinaryOp::Add);
    assert!(matches!(left.kind(), ExprKind::Name("a")));

    let (right_op, right_left, right_right) = binary(right);
    assert_eq!(right_op, BinaryOp::Mul);
    assert!(matches!(right_left.kind(), ExprKind::Name("b")));
    assert!(matches!(right_right.kind(), ExprKind::Name("c")));
}

#[test]
fn expr_parses_right_associative_concat_and_power() {
    let concat = parse("a .. b .. c");
    let (op, left, right) = binary(&concat);

    assert_eq!(op, BinaryOp::Concat);
    assert!(matches!(left.kind(), ExprKind::Name("a")));
    assert!(matches!(binary(right).0, BinaryOp::Concat));

    let power = parse("a ^ b ^ c");
    let (op, left, right) = binary(&power);
    assert_eq!(op, BinaryOp::Pow);
    assert!(matches!(left.kind(), ExprKind::Name("a")));
    assert!(matches!(binary(right).0, BinaryOp::Pow));
}

#[test]
fn expr_parses_unary_lower_than_power() {
    let expr = parse("-a ^ b");

    match expr.kind() {
        ExprKind::Unary { op, expr } => {
            assert_eq!(*op, UnaryOp::Neg);
            assert!(matches!(binary(expr).0, BinaryOp::Pow));
        }
        kind => panic!("expected unary expression, got {kind:?}"),
    }
}

#[test]
fn expr_parses_function_call_arguments() {
    let expr = parse("f(1, x)");

    match expr.kind() {
        ExprKind::Call {
            callee,
            method,
            args,
        } => {
            assert!(matches!(callee.kind(), ExprKind::Name("f")));
            assert_eq!(*method, None);
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0].kind(), ExprKind::Integer("1")));
            assert!(matches!(args[1].kind(), ExprKind::Name("x")));
        }
        kind => panic!("expected call expression, got {kind:?}"),
    }
}

#[test]
fn expr_parses_method_call_arguments() {
    let expr = parse("receiver:method('x')");

    match expr.kind() {
        ExprKind::Call {
            callee,
            method,
            args,
        } => {
            assert!(matches!(callee.kind(), ExprKind::Name("receiver")));
            assert_eq!(*method, Some("method"));
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0].kind(), ExprKind::String("'x'")));
        }
        kind => panic!("expected method call expression, got {kind:?}"),
    }
}

#[test]
fn table_access_parses_index_suffixes() {
    let expr = parse("table[key].field");

    let ExprKind::Index { table, key } = expr.kind() else {
        panic!("expected field index expression");
    };
    assert!(matches!(key.kind(), ExprKind::StringKey("field")));

    let ExprKind::Index {
        table: inner_table,
        key: inner_key,
    } = table.kind()
    else {
        panic!("expected bracket index expression");
    };
    assert!(matches!(inner_table.kind(), ExprKind::Name("table")));
    assert!(matches!(inner_key.kind(), ExprKind::Name("key")));
}

#[test]
fn expr_parses_table_and_string_call_sugar() {
    let table_call = parse("f { x = 1 }");
    let ExprKind::Call { args, .. } = table_call.kind() else {
        panic!("expected call expression");
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0].kind(), ExprKind::Table(_)));

    let string_call = parse("f \"hello\"");
    let ExprKind::Call { args, .. } = string_call.kind() else {
        panic!("expected call expression");
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0].kind(), ExprKind::String("\"hello\"")));
}

#[test]
fn expr_parses_table_constructor_fields() {
    let expr = parse("{ [key] = value, name = 1; call() }");
    let ExprKind::Table(fields) = expr.kind() else {
        panic!("expected table constructor");
    };

    assert_eq!(fields.len(), 3);
    assert!(matches!(fields[0].kind(), TableFieldKind::Keyed { .. }));
    assert!(matches!(
        fields[1].kind(),
        TableFieldKind::Named { name: "name", .. }
    ));
    assert!(matches!(fields[2].kind(), TableFieldKind::Array(_)));
}

#[test]
fn expr_parses_vararg() {
    let expr = parse("...");

    assert!(matches!(expr.kind(), ExprKind::Vararg));
}

#[test]
fn expr_reports_trailing_tokens() {
    let parsed = parse_expression(SourceId::new(0), "a b");

    assert!(parsed.expression.is_some());
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(
        parsed.diagnostics[0].message(),
        "unexpected token after expression"
    );
}

#[test]
fn expr_uses_lua_operator_tokens() {
    let parsed = parse_expression(SourceId::new(0), "a << b | c and not d");

    assert!(parsed.diagnostics.is_empty());
    assert_eq!(
        parsed.expression.as_ref().map(|expr| expr.span().source()),
        Some(SourceId::new(0))
    );
    assert_eq!(TokenKind::keyword("global"), Some(TokenKind::Global));
}
