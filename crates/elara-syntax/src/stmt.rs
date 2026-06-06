//! Lua statement parser.

use elara_core::{Diagnostic, SourceId};

use crate::{
    Block, Expr, ExprKind, FunctionBody, FunctionName, FunctionScope, GlobalDecl, IfClause,
    NameDecl, Param, Stmt, StmtKind, Token, TokenKind, lex,
};

use crate::expr::{Parser, merge_spans};

/// Result of parsing one Lua chunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedChunk<'src> {
    /// Top-level chunk block.
    pub block: Block<'src>,
    /// Lexer and parser diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses a Lua chunk.
#[must_use]
pub fn parse_chunk(source: SourceId, input: &str) -> ParsedChunk<'_> {
    let lexed = lex(source, input);
    let mut parser = Parser::new(lexed.tokens, lexed.diagnostics);
    let block = parser.parse_block_until(&[TokenKind::Eof]);
    ParsedChunk {
        block,
        diagnostics: parser.into_diagnostics(),
    }
}

impl<'src> Parser<'src> {
    fn parse_block_until(&mut self, terminators: &[TokenKind]) -> Block<'src> {
        let start = self.current_token().span();
        let mut statements = Vec::new();

        while !self.is_at_end() && !terminators.contains(&self.current_kind()) {
            statements.push(self.parse_statement());
        }

        let span = statements
            .last()
            .map_or(start, |stmt| merge_spans(start, stmt.span()));
        Block::new(statements, span)
    }

    fn parse_statement(&mut self) -> Stmt<'src> {
        match self.current_kind() {
            TokenKind::Semicolon => {
                let token = self.advance();
                Stmt::new(StmtKind::Empty, token.span())
            }
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Break => {
                let token = self.advance();
                Stmt::new(StmtKind::Break, token.span())
            }
            TokenKind::Goto => self.parse_goto_statement(),
            TokenKind::DoubleColon => self.parse_label_statement(),
            TokenKind::Local => self.parse_local_statement(),
            TokenKind::Global => self.parse_global_statement(),
            TokenKind::Function => self.parse_function_statement(FunctionScope::Plain),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Repeat => self.parse_repeat_statement(),
            TokenKind::For => self.parse_for_statement(),
            _ => self.parse_assignment_or_call_statement(),
        }
    }

    fn parse_return_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        let values = if self.is_block_terminator() || self.check(TokenKind::Semicolon) {
            Vec::new()
        } else {
            self.parse_expression_list()
        };
        let semicolon = self.match_kind(TokenKind::Semicolon);
        let span = values
            .last()
            .map_or(start.span(), |expr| merge_spans(start.span(), expr.span()));
        let span = if semicolon {
            merge_spans(start.span(), self.previous().span())
        } else {
            span
        };
        Stmt::new(StmtKind::Return(values), span)
    }

    fn parse_goto_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        let name = self.expect(TokenKind::Identifier, "expected label name after goto");
        let span = name.map_or(start.span(), |token| {
            merge_spans(start.span(), token.span())
        });
        Stmt::new(
            StmtKind::Goto(name.map_or("", |token| token.lexeme())),
            span,
        )
    }

    fn parse_label_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        let name = self.expect(TokenKind::Identifier, "expected label name");
        let end = self.expect(TokenKind::DoubleColon, "expected '::' after label");
        let span = end.or(name).map_or(start.span(), |token| {
            merge_spans(start.span(), token.span())
        });
        Stmt::new(
            StmtKind::Label(name.map_or("", |token| token.lexeme())),
            span,
        )
    }

    fn parse_local_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        if self.match_kind(TokenKind::Function) {
            return self.parse_scoped_function_after_keyword(start, FunctionScope::Local);
        }

        let prefix_attribute = self.parse_attribute();
        let names = self.parse_name_declarations(prefix_attribute);
        let values = if self.match_kind(TokenKind::Assign) {
            self.parse_expression_list()
        } else {
            Vec::new()
        };
        let span = values.last().map_or_else(
            || {
                names
                    .last()
                    .map_or(start.span(), |name| merge_spans(start.span(), name.span))
            },
            |expr| merge_spans(start.span(), expr.span()),
        );

        Stmt::new(StmtKind::Local { names, values }, span)
    }

    fn parse_global_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        if self.match_kind(TokenKind::Function) {
            return self.parse_scoped_function_after_keyword(start, FunctionScope::Global);
        }

        let prefix_attribute = self.parse_attribute();
        if self.match_kind(TokenKind::Star) {
            return Stmt::new(
                StmtKind::Global(GlobalDecl::All {
                    attribute: prefix_attribute,
                }),
                merge_spans(start.span(), self.previous().span()),
            );
        }

        let names = self.parse_name_declarations(prefix_attribute);
        let values = if self.match_kind(TokenKind::Assign) {
            self.parse_expression_list()
        } else {
            Vec::new()
        };
        let span = values.last().map_or_else(
            || {
                names
                    .last()
                    .map_or(start.span(), |name| merge_spans(start.span(), name.span))
            },
            |expr| merge_spans(start.span(), expr.span()),
        );

        Stmt::new(StmtKind::Global(GlobalDecl::Names { names, values }), span)
    }

    fn parse_function_statement(&mut self, scope: FunctionScope) -> Stmt<'src> {
        let start = self.advance();
        self.parse_scoped_function_after_keyword(start, scope)
    }

    fn parse_scoped_function_after_keyword(
        &mut self,
        start: Token<'src>,
        scope: FunctionScope,
    ) -> Stmt<'src> {
        let name = self.parse_function_name();
        let body = self.parse_function_body();
        let span = merge_spans(start.span(), body.span);
        Stmt::new(StmtKind::Function { scope, name, body }, span)
    }

    fn parse_if_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        let condition = self.parse_expression(1);
        self.expect(TokenKind::Then, "expected 'then' after condition");
        let block = self.parse_block_until(&[TokenKind::ElseIf, TokenKind::Else, TokenKind::End]);
        let mut clauses = vec![IfClause { condition, block }];

        while self.match_kind(TokenKind::ElseIf) {
            let condition = self.parse_expression(1);
            self.expect(TokenKind::Then, "expected 'then' after elseif condition");
            let block =
                self.parse_block_until(&[TokenKind::ElseIf, TokenKind::Else, TokenKind::End]);
            clauses.push(IfClause { condition, block });
        }

        let else_block = if self.match_kind(TokenKind::Else) {
            Some(self.parse_block_until(&[TokenKind::End]))
        } else {
            None
        };
        let end = self.expect(TokenKind::End, "expected 'end' after if statement");
        let span = end.map_or(start.span(), |token| {
            merge_spans(start.span(), token.span())
        });
        Stmt::new(
            StmtKind::If {
                clauses,
                else_block,
            },
            span,
        )
    }

    fn parse_while_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        let condition = self.parse_expression(1);
        self.expect(TokenKind::Do, "expected 'do' after while condition");
        let body = self.parse_block_until(&[TokenKind::End]);
        let end = self.expect(TokenKind::End, "expected 'end' after while statement");
        let span = end.map_or(start.span(), |token| {
            merge_spans(start.span(), token.span())
        });
        Stmt::new(StmtKind::While { condition, body }, span)
    }

    fn parse_repeat_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        let body = self.parse_block_until(&[TokenKind::Until]);
        self.expect(TokenKind::Until, "expected 'until' after repeat block");
        let condition = self.parse_expression(1);
        let span = merge_spans(start.span(), condition.span());
        Stmt::new(StmtKind::Repeat { body, condition }, span)
    }

    fn parse_for_statement(&mut self) -> Stmt<'src> {
        let start = self.advance();
        let name = self.expect(TokenKind::Identifier, "expected for-loop variable");
        let Some(first_name) = name else {
            return Stmt::new(StmtKind::Break, start.span());
        };

        if self.match_kind(TokenKind::Assign) {
            let init = self.parse_expression(1);
            self.expect(TokenKind::Comma, "expected ',' after initial value");
            let limit = self.parse_expression(1);
            let step = if self.match_kind(TokenKind::Comma) {
                Some(self.parse_expression(1))
            } else {
                None
            };
            self.expect(TokenKind::Do, "expected 'do' after numeric for header");
            let body = self.parse_block_until(&[TokenKind::End]);
            let end = self.expect(TokenKind::End, "expected 'end' after for statement");
            let span = end.map_or(start.span(), |token| {
                merge_spans(start.span(), token.span())
            });
            return Stmt::new(
                StmtKind::NumericFor {
                    name: first_name.lexeme(),
                    init,
                    limit,
                    step,
                    body,
                },
                span,
            );
        }

        let mut names = vec![first_name.lexeme()];
        while self.match_kind(TokenKind::Comma) {
            if let Some(name) = self.expect(TokenKind::Identifier, "expected for-loop variable") {
                names.push(name.lexeme());
            }
        }
        self.expect(TokenKind::In, "expected 'in' after generic for variables");
        let values = self.parse_expression_list();
        self.expect(TokenKind::Do, "expected 'do' after generic for header");
        let body = self.parse_block_until(&[TokenKind::End]);
        let end = self.expect(TokenKind::End, "expected 'end' after for statement");
        let span = end.map_or(start.span(), |token| {
            merge_spans(start.span(), token.span())
        });
        Stmt::new(
            StmtKind::GenericFor {
                names,
                values,
                body,
            },
            span,
        )
    }

    fn parse_assignment_or_call_statement(&mut self) -> Stmt<'src> {
        let start = self.current_token();
        let first = self.parse_expression(1);

        if matches!(first.kind(), ExprKind::Call { .. })
            && !self.check(TokenKind::Comma)
            && !self.check(TokenKind::Assign)
        {
            return Stmt::new(StmtKind::Call(first.clone()), first.span());
        }

        let mut targets = vec![first];
        while self.match_kind(TokenKind::Comma) {
            targets.push(self.parse_expression(1));
        }
        self.expect(TokenKind::Assign, "expected '=' after assignment targets");
        let values = self.parse_expression_list();
        let span = values
            .last()
            .map_or(start.span(), |expr| merge_spans(start.span(), expr.span()));
        Stmt::new(StmtKind::Assign { targets, values }, span)
    }

    fn parse_function_name(&mut self) -> FunctionName<'src> {
        let start = self.expect(TokenKind::Identifier, "expected function name");
        let mut fields = Vec::new();
        while self.match_kind(TokenKind::Dot) {
            if let Some(field) = self.expect(TokenKind::Identifier, "expected function field name")
            {
                fields.push(field.lexeme());
            }
        }
        let method = if self.match_kind(TokenKind::Colon) {
            self.expect(TokenKind::Identifier, "expected method name")
                .map(|token| token.lexeme())
        } else {
            None
        };
        let end = self.previous();
        FunctionName {
            base: start.map_or("", |token| token.lexeme()),
            fields,
            method,
            span: start.map_or(end.span(), |token| merge_spans(token.span(), end.span())),
        }
    }

    fn parse_function_body(&mut self) -> FunctionBody<'src> {
        let open = self.expect(TokenKind::LeftParen, "expected '(' before parameters");
        let params = self.parse_params();
        let close = self.expect(TokenKind::RightParen, "expected ')' after parameters");
        let block = self.parse_block_until(&[TokenKind::End]);
        let end = self.expect(TokenKind::End, "expected 'end' after function body");
        let start_span = open
            .or(close)
            .map_or_else(|| block.span(), |token| token.span());
        let end_span = end.map_or_else(|| block.span(), |token| token.span());
        FunctionBody {
            params,
            block,
            span: merge_spans(start_span, end_span),
        }
    }

    fn parse_params(&mut self) -> Vec<Param<'src>> {
        let mut params = Vec::new();
        if self.check(TokenKind::RightParen) || self.check(TokenKind::Eof) {
            return params;
        }

        loop {
            if self.match_kind(TokenKind::DotDotDot) {
                let name = if self.check(TokenKind::Identifier) {
                    Some(self.advance().lexeme())
                } else {
                    None
                };
                params.push(Param::Vararg(name));
                break;
            }

            if let Some(name) = self.expect(TokenKind::Identifier, "expected parameter name") {
                params.push(Param::Name(name.lexeme()));
            }

            if !self.match_kind(TokenKind::Comma) {
                break;
            }
        }

        params
    }

    fn parse_name_declarations(
        &mut self,
        prefix_attribute: Option<&'src str>,
    ) -> Vec<NameDecl<'src>> {
        let mut names = Vec::new();
        while let Some(name) = self.expect(TokenKind::Identifier, "expected declaration name") {
            let attribute = self.parse_attribute().or(prefix_attribute);
            names.push(NameDecl {
                name: name.lexeme(),
                attribute,
                span: name.span(),
            });

            if !self.match_kind(TokenKind::Comma) {
                break;
            }
        }
        names
    }

    fn parse_attribute(&mut self) -> Option<&'src str> {
        if !self.match_kind(TokenKind::Less) {
            return None;
        }
        let name = self.expect(TokenKind::Identifier, "expected attribute name");
        self.expect(TokenKind::Greater, "expected '>' after attribute");
        name.map(|token| token.lexeme())
    }

    fn parse_expression_list(&mut self) -> Vec<Expr<'src>> {
        let mut values = vec![self.parse_expression(1)];
        while self.match_kind(TokenKind::Comma) {
            values.push(self.parse_expression(1));
        }
        values
    }

    fn is_block_terminator(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Eof
                | TokenKind::End
                | TokenKind::Else
                | TokenKind::ElseIf
                | TokenKind::Until
        )
    }
}

#[cfg(test)]
mod tests;
