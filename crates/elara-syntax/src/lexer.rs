//! Lua lexer.

use elara_core::{Diagnostic, SourceId, Span};

use crate::{Token, TokenKind};

/// Result of lexing one Lua source document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lexed<'src> {
    /// Tokens, including one EOF sentinel.
    pub tokens: Vec<Token<'src>>,
    /// Diagnostics emitted while scanning.
    pub diagnostics: Vec<Diagnostic>,
}

/// Lexes Lua source into tokens.
#[must_use]
pub fn lex(source: SourceId, input: &str) -> Lexed<'_> {
    let mut lexer = Lexer::new(source, input);
    lexer.lex_all();
    lexer.finish()
}

struct Lexer<'src> {
    source: SourceId,
    input: &'src str,
    current: usize,
    tokens: Vec<Token<'src>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Lexer<'src> {
    fn new(source: SourceId, input: &'src str) -> Self {
        Self {
            source,
            input,
            current: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn lex_all(&mut self) {
        while !self.is_at_end() {
            let start = self.current;
            let byte = self.advance();
            match byte {
                b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x0b' => {}
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_identifier(start),
                b'0'..=b'9' => self.lex_number(start, false),
                b'\'' | b'"' => self.lex_short_string(start, byte),
                b'-' if self.match_byte(b'-') => self.skip_comment(start),
                b'.' if self.peek().is_some_and(is_decimal_digit) => {
                    self.lex_number(start, true);
                }
                b'.' => self.lex_dot(start),
                b'[' => {
                    if self.long_bracket_level(start).is_some() {
                        self.current = start;
                        self.lex_long_string(start);
                    } else {
                        self.push_token(TokenKind::LeftBracket, start, self.current);
                    }
                }
                b'+' => self.push_token(TokenKind::Plus, start, self.current),
                b'-' => self.push_token(TokenKind::Minus, start, self.current),
                b'*' => self.push_token(TokenKind::Star, start, self.current),
                b'/' if self.match_byte(b'/') => {
                    self.push_token(TokenKind::FloorDiv, start, self.current);
                }
                b'/' => self.push_token(TokenKind::Slash, start, self.current),
                b'%' => self.push_token(TokenKind::Percent, start, self.current),
                b'^' => self.push_token(TokenKind::Caret, start, self.current),
                b'#' => self.push_token(TokenKind::Hash, start, self.current),
                b'&' => self.push_token(TokenKind::Ampersand, start, self.current),
                b'|' => self.push_token(TokenKind::Pipe, start, self.current),
                b'~' if self.match_byte(b'=') => {
                    self.push_token(TokenKind::TildeEqual, start, self.current);
                }
                b'~' => self.push_token(TokenKind::Tilde, start, self.current),
                b'<' if self.match_byte(b'<') => {
                    self.push_token(TokenKind::ShiftLeft, start, self.current);
                }
                b'<' if self.match_byte(b'=') => {
                    self.push_token(TokenKind::LessEqual, start, self.current);
                }
                b'<' => self.push_token(TokenKind::Less, start, self.current),
                b'>' if self.match_byte(b'>') => {
                    self.push_token(TokenKind::ShiftRight, start, self.current);
                }
                b'>' if self.match_byte(b'=') => {
                    self.push_token(TokenKind::GreaterEqual, start, self.current);
                }
                b'>' => self.push_token(TokenKind::Greater, start, self.current),
                b'=' if self.match_byte(b'=') => {
                    self.push_token(TokenKind::EqualEqual, start, self.current);
                }
                b'=' => self.push_token(TokenKind::Assign, start, self.current),
                b'(' => self.push_token(TokenKind::LeftParen, start, self.current),
                b')' => self.push_token(TokenKind::RightParen, start, self.current),
                b'{' => self.push_token(TokenKind::LeftBrace, start, self.current),
                b'}' => self.push_token(TokenKind::RightBrace, start, self.current),
                b']' => self.push_token(TokenKind::RightBracket, start, self.current),
                b':' if self.match_byte(b':') => {
                    self.push_token(TokenKind::DoubleColon, start, self.current);
                }
                b':' => self.push_token(TokenKind::Colon, start, self.current),
                b';' => self.push_token(TokenKind::Semicolon, start, self.current),
                b',' => self.push_token(TokenKind::Comma, start, self.current),
                _ => self.error_span(start, self.current, "invalid character"),
            }
        }
    }

    fn finish(mut self) -> Lexed<'src> {
        self.push_token(TokenKind::Eof, self.current, self.current);
        Lexed {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn lex_identifier(&mut self, start: usize) {
        while self.peek().is_some_and(is_identifier_continue) {
            self.advance();
        }
        let text = &self.input[start..self.current];
        let kind = TokenKind::keyword(text).unwrap_or(TokenKind::Identifier);
        self.push_token(kind, start, self.current);
    }

    fn lex_number(&mut self, start: usize, leading_dot: bool) {
        let mut is_float = leading_dot;
        let mut valid = true;

        if !leading_dot
            && (self.input[start..].starts_with("0x") || self.input[start..].starts_with("0X"))
        {
            self.advance();
            valid &= self.consume_while_count(is_hex_digit) > 0;
            if self.peek() == Some(b'.') && self.peek_next() != Some(b'.') {
                is_float = true;
                self.advance();
                self.consume_while_count(is_hex_digit);
            }
            if matches!(self.peek(), Some(b'p' | b'P')) {
                is_float = true;
                self.advance();
                self.consume_sign();
                valid &= self.consume_while_count(is_decimal_digit) > 0;
            }
        } else {
            self.consume_while_count(is_decimal_digit);
            if self.peek() == Some(b'.') && self.peek_next() != Some(b'.') {
                is_float = true;
                self.advance();
                self.consume_while_count(is_decimal_digit);
            }
            if matches!(self.peek(), Some(b'e' | b'E')) {
                is_float = true;
                self.advance();
                self.consume_sign();
                valid &= self.consume_while_count(is_decimal_digit) > 0;
            }
        }

        if self.peek().is_some_and(is_identifier_start) {
            valid = false;
            self.consume_while_count(is_identifier_continue);
        }

        if !valid {
            self.error_span(start, self.current, "malformed numeral");
        }

        let kind = if is_float {
            TokenKind::Float
        } else {
            TokenKind::Integer
        };
        self.push_token(kind, start, self.current);
    }

    fn lex_short_string(&mut self, start: usize, quote: u8) {
        let mut terminated = false;
        while let Some(byte) = self.peek() {
            match byte {
                b if b == quote => {
                    self.advance();
                    terminated = true;
                    break;
                }
                b'\n' | b'\r' => {
                    self.error_span(start, self.current, "unterminated string literal");
                    break;
                }
                b'\\' => self.lex_escape(),
                _ => {
                    self.advance();
                }
            }
        }

        if !terminated && self.is_at_end() {
            self.error_span(start, self.current, "unterminated string literal");
        }

        self.push_token(TokenKind::String, start, self.current);
    }

    fn lex_escape(&mut self) {
        let slash = self.current;
        self.advance();
        let Some(escaped) = self.advance_if_present() else {
            self.error_span(slash, self.current, "unterminated escape sequence");
            return;
        };

        match escaped {
            b'a' | b'b' | b'f' | b'n' | b'r' | b't' | b'v' | b'\\' | b'"' | b'\'' => {}
            b'z' => {
                while let Some(byte) = self.peek()
                    && matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x0b')
                {
                    if matches!(byte, b'\n' | b'\r') {
                        self.consume_newline();
                    } else {
                        self.advance();
                    }
                }
            }
            b'\n' | b'\r' => {}
            b'x' => {
                if !self.consume_exact_digits(2, is_hex_digit) {
                    self.error_span(slash, self.current, "invalid hexadecimal escape");
                }
            }
            b'u' => self.lex_unicode_escape(slash),
            b'0'..=b'9' => {
                for _ in 0..2 {
                    if self.peek().is_some_and(is_decimal_digit) {
                        self.advance();
                    }
                }
            }
            _ => self.error_span(slash, self.current, "invalid escape sequence"),
        }
    }

    fn lex_unicode_escape(&mut self, slash: usize) {
        if !self.match_byte(b'{') {
            self.error_span(slash, self.current, "invalid unicode escape");
            return;
        }

        let digits = self.consume_while_count(is_hex_digit);
        if digits == 0 || !self.match_byte(b'}') {
            self.error_span(slash, self.current, "invalid unicode escape");
        }
    }

    fn lex_long_string(&mut self, start: usize) {
        let Some(level) = self.consume_long_bracket_start() else {
            return;
        };
        if matches!(self.peek(), Some(b'\n' | b'\r')) {
            self.consume_newline();
        }

        while !self.is_at_end() {
            if self.is_long_bracket_close(level) {
                self.consume_long_bracket_close(level);
                self.push_token(TokenKind::String, start, self.current);
                return;
            }
            self.advance();
        }

        self.error_span(start, self.current, "unterminated long string literal");
        self.push_token(TokenKind::String, start, self.current);
    }

    fn skip_comment(&mut self, start: usize) {
        if self.long_bracket_level(self.current).is_some() {
            let Some(level) = self.consume_long_bracket_start() else {
                return;
            };
            while !self.is_at_end() {
                if self.is_long_bracket_close(level) {
                    self.consume_long_bracket_close(level);
                    return;
                }
                self.advance();
            }
            self.error_span(start, self.current, "unterminated long comment");
            return;
        }

        while !self.is_at_end() && !matches!(self.peek(), Some(b'\n' | b'\r')) {
            self.advance();
        }
    }

    fn lex_dot(&mut self, start: usize) {
        if self.match_byte(b'.') {
            if self.match_byte(b'.') {
                self.push_token(TokenKind::DotDotDot, start, self.current);
            } else {
                self.push_token(TokenKind::DotDot, start, self.current);
            }
        } else {
            self.push_token(TokenKind::Dot, start, self.current);
        }
    }

    fn push_token(&mut self, kind: TokenKind, start: usize, end: usize) {
        let span = self.span(start, end);
        self.tokens
            .push(Token::new(kind, &self.input[start..end], span));
    }

    fn error_span(&mut self, start: usize, end: usize, message: &'static str) {
        self.diagnostics
            .push(Diagnostic::error(message).with_primary_span(self.span(start, end)));
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.source, to_u32(start), to_u32(end))
    }

    fn long_bracket_level(&self, start: usize) -> Option<usize> {
        let bytes = self.input.as_bytes();
        if bytes.get(start) != Some(&b'[') {
            return None;
        }

        let mut cursor = start + 1;
        while bytes.get(cursor) == Some(&b'=') {
            cursor += 1;
        }
        (bytes.get(cursor) == Some(&b'[')).then_some(cursor - start - 1)
    }

    fn consume_long_bracket_start(&mut self) -> Option<usize> {
        let level = self.long_bracket_level(self.current)?;
        self.current += 2 + level;
        Some(level)
    }

    fn is_long_bracket_close(&self, level: usize) -> bool {
        let bytes = self.input.as_bytes();
        if bytes.get(self.current) != Some(&b']') {
            return false;
        }
        let equals_start = self.current + 1;
        let close = equals_start + level;
        bytes.get(equals_start..close).is_some_and(|slice| {
            slice.iter().all(|byte| *byte == b'=') && bytes.get(close) == Some(&b']')
        })
    }

    fn consume_long_bracket_close(&mut self, level: usize) {
        self.current += 2 + level;
    }

    fn consume_newline(&mut self) {
        let first = self.advance();
        if (first == b'\r' && self.peek() == Some(b'\n'))
            || (first == b'\n' && self.peek() == Some(b'\r'))
        {
            self.advance();
        }
    }

    fn consume_sign(&mut self) {
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.advance();
        }
    }

    fn consume_while_count(&mut self, predicate: impl Fn(u8) -> bool) -> usize {
        let start = self.current;
        while self.peek().is_some_and(&predicate) {
            self.advance();
        }
        self.current - start
    }

    fn consume_exact_digits(&mut self, count: usize, predicate: impl Fn(u8) -> bool) -> bool {
        for _ in 0..count {
            if self.peek().is_some_and(&predicate) {
                self.advance();
            } else {
                return false;
            }
        }
        true
    }

    fn match_byte(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance_if_present(&mut self) -> Option<u8> {
        (!self.is_at_end()).then(|| self.advance())
    }

    fn advance(&mut self) -> u8 {
        let byte = self.input.as_bytes()[self.current];
        self.current += 1;
        byte
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.current).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.input.as_bytes().get(self.current + 1).copied()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.input.len()
    }
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || is_decimal_digit(byte)
}

fn is_decimal_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

fn is_hex_digit(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).expect("source offset must fit in u32")
}

#[cfg(test)]
mod tests {
    use elara_core::SourceId;

    use crate::{TokenKind, lex};

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(SourceId::new(0), source)
            .tokens
            .into_iter()
            .map(|token| token.kind())
            .collect()
    }

    fn lexemes(source: &str) -> Vec<&str> {
        lex(SourceId::new(0), source)
            .tokens
            .into_iter()
            .map(|token| token.lexeme())
            .collect()
    }

    #[test]
    fn lexer_keywords_and_identifiers() {
        let lexed = lex(SourceId::new(0), "global local and And _name name1");

        assert!(lexed.diagnostics.is_empty());
        assert_eq!(
            lexed
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Global,
                TokenKind::Local,
                TokenKind::And,
                TokenKind::Identifier,
                TokenKind::Identifier,
                TokenKind::Identifier,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexer_punctuation_and_operators() {
        assert_eq!(
            kinds("+ - * / // % ^ # & ~ | << >> == ~= <= >= < > = ( ) { } [ ] :: ; : , . .. ..."),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::FloorDiv,
                TokenKind::Percent,
                TokenKind::Caret,
                TokenKind::Hash,
                TokenKind::Ampersand,
                TokenKind::Tilde,
                TokenKind::Pipe,
                TokenKind::ShiftLeft,
                TokenKind::ShiftRight,
                TokenKind::EqualEqual,
                TokenKind::TildeEqual,
                TokenKind::LessEqual,
                TokenKind::GreaterEqual,
                TokenKind::Less,
                TokenKind::Greater,
                TokenKind::Assign,
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::DoubleColon,
                TokenKind::Semicolon,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::DotDot,
                TokenKind::DotDotDot,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexer_numbers() {
        let lexed = lex(SourceId::new(0), "3 3.0 34e1 0xff 0xA23p-4 .5 3..4");

        assert!(lexed.diagnostics.is_empty());
        assert_eq!(
            lexed
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Integer,
                TokenKind::Float,
                TokenKind::Float,
                TokenKind::Integer,
                TokenKind::Float,
                TokenKind::Float,
                TokenKind::Integer,
                TokenKind::DotDot,
                TokenKind::Integer,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexer_strings_and_comments() {
        let source = "local s = 'a\\n' -- ignored\nlocal t = [=[long]=] --[=[ also ignored ]=]";
        let lexed = lex(SourceId::new(0), source);

        assert!(lexed.diagnostics.is_empty());
        assert_eq!(
            lexed
                .tokens
                .iter()
                .map(|token| token.kind())
                .collect::<Vec<_>>(),
            vec![
                TokenKind::Local,
                TokenKind::Identifier,
                TokenKind::Assign,
                TokenKind::String,
                TokenKind::Local,
                TokenKind::Identifier,
                TokenKind::Assign,
                TokenKind::String,
                TokenKind::Eof,
            ]
        );
        assert_eq!(lexemes(source)[3], "'a\\n'");
    }

    #[test]
    fn lexer_reports_invalid_input() {
        let lexed = lex(SourceId::new(7), "'unterminated\n@ 0x");

        assert_eq!(lexed.diagnostics.len(), 3);
        assert_eq!(
            lexed.diagnostics[0].message(),
            "unterminated string literal"
        );
        assert_eq!(
            lexed.diagnostics[0].primary_span().unwrap().source(),
            SourceId::new(7)
        );
        assert_eq!(lexed.diagnostics[1].message(), "invalid character");
        assert_eq!(lexed.diagnostics[2].message(), "malformed numeral");
    }
}
