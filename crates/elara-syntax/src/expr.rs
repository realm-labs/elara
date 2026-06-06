//! Lua expression parser.

use elara_core::{Diagnostic, SourceId, Span};

use crate::{BinaryOp, Expr, ExprKind, TableField, TableFieldKind, Token, TokenKind, UnaryOp, lex};

const UNARY_PRECEDENCE: u8 = 11;

/// Result of parsing one expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedExpression<'src> {
    /// Parsed expression when one could be produced.
    pub expression: Option<Expr<'src>>,
    /// Lexer and parser diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses one Lua expression from source text.
#[must_use]
pub fn parse_expression(source: SourceId, input: &str) -> ParsedExpression<'_> {
    let lexed = lex(source, input);
    let parser = Parser::new(lexed.tokens, lexed.diagnostics);
    parser.parse_top_level_expression()
}

pub(crate) struct Parser<'src> {
    tokens: Vec<Token<'src>>,
    current: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Parser<'src> {
    pub(crate) fn new(tokens: Vec<Token<'src>>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            tokens,
            current: 0,
            diagnostics,
        }
    }

    fn parse_top_level_expression(mut self) -> ParsedExpression<'src> {
        let expression = if self.check(TokenKind::Eof) {
            self.error_current("expected expression");
            None
        } else {
            Some(self.parse_expression(1))
        };

        if !self.check(TokenKind::Eof) {
            self.error_current("unexpected token after expression");
        }

        ParsedExpression {
            expression,
            diagnostics: self.diagnostics,
        }
    }

    pub(crate) fn parse_expression(&mut self, min_precedence: u8) -> Expr<'src> {
        let mut left = if let Some(op) = self.current_unary_op() {
            let op_token = self.advance();
            let expr = self.parse_expression(UNARY_PRECEDENCE);
            let span = merge_spans(op_token.span(), expr.span());
            Expr::new(
                ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span,
            )
        } else {
            self.parse_prefix_expression()
        };

        while let Some(binary) = self.current_binary_op() {
            if binary.precedence < min_precedence {
                break;
            }

            self.advance();
            let next_min = if binary.right_associative {
                binary.precedence
            } else {
                binary.precedence + 1
            };
            let right = self.parse_expression(next_min);
            let span = merge_spans(left.span(), right.span());
            left = Expr::new(
                ExprKind::Binary {
                    op: binary.op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    fn parse_prefix_expression(&mut self) -> Expr<'src> {
        let mut expr = self.parse_primary();

        loop {
            if self.starts_args() {
                let args = self.parse_args();
                let span = args
                    .last()
                    .map_or_else(|| expr.span(), |last| merge_spans(expr.span(), last.span()));
                expr = Expr::new(
                    ExprKind::Call {
                        callee: Box::new(expr),
                        method: None,
                        args,
                    },
                    span,
                );
            } else if self.match_kind(TokenKind::Colon) {
                let method = self.expect(TokenKind::Identifier, "expected method name");
                let args = if self.starts_args() {
                    self.parse_args()
                } else {
                    self.error_current("expected method call arguments");
                    Vec::new()
                };
                let method_name = method.map(|token| token.lexeme());
                let start_span = expr.span();
                let end_span = args.last().map_or_else(
                    || method.map_or(start_span, |token| token.span()),
                    |arg| arg.span(),
                );
                expr = Expr::new(
                    ExprKind::Call {
                        callee: Box::new(expr),
                        method: method_name,
                        args,
                    },
                    merge_spans(start_span, end_span),
                );
            } else {
                break;
            }
        }

        expr
    }

    fn parse_primary(&mut self) -> Expr<'src> {
        let token = self.advance();
        match token.kind() {
            TokenKind::Nil => Expr::new(ExprKind::Nil, token.span()),
            TokenKind::False => Expr::new(ExprKind::Bool(false), token.span()),
            TokenKind::True => Expr::new(ExprKind::Bool(true), token.span()),
            TokenKind::Integer => Expr::new(ExprKind::Integer(token.lexeme()), token.span()),
            TokenKind::Float => Expr::new(ExprKind::Float(token.lexeme()), token.span()),
            TokenKind::String => Expr::new(ExprKind::String(token.lexeme()), token.span()),
            TokenKind::Identifier => Expr::new(ExprKind::Name(token.lexeme()), token.span()),
            TokenKind::DotDotDot => Expr::new(ExprKind::Vararg, token.span()),
            TokenKind::LeftParen => self.parse_grouped(token),
            TokenKind::LeftBrace => self.parse_table_constructor(token),
            _ => {
                self.error_at(token, "expected expression");
                Expr::new(ExprKind::Error, token.span())
            }
        }
    }

    fn parse_grouped(&mut self, open: Token<'src>) -> Expr<'src> {
        let inner = self.parse_expression(1);
        let close = self.expect(TokenKind::RightParen, "expected ')' after expression");
        let span = close.map_or_else(
            || merge_spans(open.span(), inner.span()),
            |token| merge_spans(open.span(), token.span()),
        );
        Expr::new(ExprKind::Grouped(Box::new(inner)), span)
    }

    fn parse_table_constructor(&mut self, open: Token<'src>) -> Expr<'src> {
        let mut fields = Vec::new();

        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            fields.push(self.parse_table_field());

            if self.match_kind(TokenKind::Comma) || self.match_kind(TokenKind::Semicolon) {
                continue;
            }

            if !self.check(TokenKind::RightBrace) {
                self.error_current("expected field separator");
                break;
            }
        }

        let close = self.expect(
            TokenKind::RightBrace,
            "expected '}' after table constructor",
        );
        let span = close.map_or_else(
            || {
                fields.last().map_or_else(
                    || open.span(),
                    |field| merge_spans(open.span(), field.span()),
                )
            },
            |token| merge_spans(open.span(), token.span()),
        );
        Expr::new(ExprKind::Table(fields), span)
    }

    fn parse_table_field(&mut self) -> TableField<'src> {
        if self.match_kind(TokenKind::LeftBracket) {
            let open = self.previous();
            let key = self.parse_expression(1);
            self.expect(TokenKind::RightBracket, "expected ']' after table key");
            self.expect(TokenKind::Assign, "expected '=' after table key");
            let value = self.parse_expression(1);
            let span = merge_spans(open.span(), value.span());
            return TableField::new(TableFieldKind::Keyed { key, value }, span);
        }

        if self.check(TokenKind::Identifier) && self.check_next(TokenKind::Assign) {
            let name = self.advance();
            self.advance();
            let value = self.parse_expression(1);
            let span = merge_spans(name.span(), value.span());
            return TableField::new(
                TableFieldKind::Named {
                    name: name.lexeme(),
                    value,
                },
                span,
            );
        }

        let value = self.parse_expression(1);
        let span = value.span();
        TableField::new(TableFieldKind::Array(value), span)
    }

    fn parse_args(&mut self) -> Vec<Expr<'src>> {
        if self.match_kind(TokenKind::LeftParen) {
            if self.check(TokenKind::RightParen) {
                self.advance();
                return Vec::new();
            }

            let mut args = Vec::new();
            loop {
                args.push(self.parse_expression(1));
                if !self.match_kind(TokenKind::Comma) {
                    break;
                }
                if self.check(TokenKind::RightParen) {
                    break;
                }
            }
            self.expect(TokenKind::RightParen, "expected ')' after arguments");
            return args;
        }

        if self.check(TokenKind::LeftBrace) {
            let open = self.advance();
            return vec![self.parse_table_constructor(open)];
        }

        if self.check(TokenKind::String) {
            let string = self.advance();
            return vec![Expr::new(ExprKind::String(string.lexeme()), string.span())];
        }

        self.error_current("expected call arguments");
        Vec::new()
    }

    fn starts_args(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::LeftParen | TokenKind::LeftBrace | TokenKind::String
        )
    }

    fn current_unary_op(&self) -> Option<UnaryOp> {
        match self.current_kind() {
            TokenKind::Not => Some(UnaryOp::Not),
            TokenKind::Hash => Some(UnaryOp::Len),
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            _ => None,
        }
    }

    fn current_binary_op(&self) -> Option<BinaryInfo> {
        let (op, precedence, right_associative) = match self.current_kind() {
            TokenKind::Or => (BinaryOp::Or, 1, false),
            TokenKind::And => (BinaryOp::And, 2, false),
            TokenKind::EqualEqual => (BinaryOp::Eq, 3, false),
            TokenKind::TildeEqual => (BinaryOp::Ne, 3, false),
            TokenKind::Less => (BinaryOp::Lt, 3, false),
            TokenKind::LessEqual => (BinaryOp::Le, 3, false),
            TokenKind::Greater => (BinaryOp::Gt, 3, false),
            TokenKind::GreaterEqual => (BinaryOp::Ge, 3, false),
            TokenKind::Pipe => (BinaryOp::BitOr, 4, false),
            TokenKind::Tilde => (BinaryOp::BitXor, 5, false),
            TokenKind::Ampersand => (BinaryOp::BitAnd, 6, false),
            TokenKind::ShiftLeft => (BinaryOp::ShiftLeft, 7, false),
            TokenKind::ShiftRight => (BinaryOp::ShiftRight, 7, false),
            TokenKind::DotDot => (BinaryOp::Concat, 8, true),
            TokenKind::Plus => (BinaryOp::Add, 9, false),
            TokenKind::Minus => (BinaryOp::Sub, 9, false),
            TokenKind::Star => (BinaryOp::Mul, 10, false),
            TokenKind::Slash => (BinaryOp::Div, 10, false),
            TokenKind::FloorDiv => (BinaryOp::FloorDiv, 10, false),
            TokenKind::Percent => (BinaryOp::Mod, 10, false),
            TokenKind::Caret => (BinaryOp::Pow, 12, true),
            _ => return None,
        };
        Some(BinaryInfo {
            op,
            precedence,
            right_associative,
        })
    }

    pub(crate) fn expect(&mut self, kind: TokenKind, message: &'static str) -> Option<Token<'src>> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            self.error_current(message);
            None
        }
    }

    pub(crate) fn match_kind(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn check(&self, kind: TokenKind) -> bool {
        self.current_kind() == kind
    }

    pub(crate) fn check_next(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.current + 1)
            .is_some_and(|token| token.kind() == kind)
    }

    pub(crate) fn current_kind(&self) -> TokenKind {
        self.tokens[self.current].kind()
    }

    pub(crate) fn advance(&mut self) -> Token<'src> {
        let token = self.tokens[self.current];
        if token.kind() != TokenKind::Eof {
            self.current += 1;
        }
        token
    }

    pub(crate) fn previous(&self) -> Token<'src> {
        self.tokens[self.current - 1]
    }

    pub(crate) fn error_current(&mut self, message: &'static str) {
        self.error_at(self.tokens[self.current], message);
    }

    pub(crate) fn error_at(&mut self, token: Token<'src>, message: &'static str) {
        self.diagnostics
            .push(Diagnostic::error(message).with_primary_span(token.span()));
    }

    pub(crate) fn current_token(&self) -> Token<'src> {
        self.tokens[self.current]
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.check(TokenKind::Eof)
    }

    pub(crate) fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

#[derive(Clone, Copy)]
struct BinaryInfo {
    op: BinaryOp,
    precedence: u8,
    right_associative: bool,
}

pub(crate) fn merge_spans(left: Span, right: Span) -> Span {
    Span::new(left.source(), left.start(), right.end())
}

#[cfg(test)]
mod tests;
