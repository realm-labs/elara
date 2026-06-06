//! Lua lexical token model.

use elara_core::Span;

/// Token kind produced by the Lua lexer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TokenKind {
    /// End-of-file sentinel.
    Eof,
    /// Identifier that is not a reserved keyword.
    Identifier,
    /// Integer numeral.
    Integer,
    /// Floating-point numeral.
    Float,
    /// String literal.
    String,
    /// `and`.
    And,
    /// `break`.
    Break,
    /// `do`.
    Do,
    /// `else`.
    Else,
    /// `elseif`.
    ElseIf,
    /// `end`.
    End,
    /// `false`.
    False,
    /// `for`.
    For,
    /// `function`.
    Function,
    /// `global`.
    Global,
    /// `goto`.
    Goto,
    /// `if`.
    If,
    /// `in`.
    In,
    /// `local`.
    Local,
    /// `nil`.
    Nil,
    /// `not`.
    Not,
    /// `or`.
    Or,
    /// `repeat`.
    Repeat,
    /// `return`.
    Return,
    /// `then`.
    Then,
    /// `true`.
    True,
    /// `until`.
    Until,
    /// `while`.
    While,
    /// `+`.
    Plus,
    /// `-`.
    Minus,
    /// `*`.
    Star,
    /// `/`.
    Slash,
    /// `%`.
    Percent,
    /// `^`.
    Caret,
    /// `#`.
    Hash,
    /// `&`.
    Ampersand,
    /// `~`.
    Tilde,
    /// `|`.
    Pipe,
    /// `<<`.
    ShiftLeft,
    /// `>>`.
    ShiftRight,
    /// `//`.
    FloorDiv,
    /// `==`.
    EqualEqual,
    /// `~=`.
    TildeEqual,
    /// `<=`.
    LessEqual,
    /// `>=`.
    GreaterEqual,
    /// `<`.
    Less,
    /// `>`.
    Greater,
    /// `=`.
    Assign,
    /// `(`.
    LeftParen,
    /// `)`.
    RightParen,
    /// `{`.
    LeftBrace,
    /// `}`.
    RightBrace,
    /// `[`.
    LeftBracket,
    /// `]`.
    RightBracket,
    /// `::`.
    DoubleColon,
    /// `;`.
    Semicolon,
    /// `:`.
    Colon,
    /// `,`.
    Comma,
    /// `.`.
    Dot,
    /// `..`.
    DotDot,
    /// `...`.
    DotDotDot,
}

impl TokenKind {
    /// Returns the token kind for a reserved keyword.
    #[must_use]
    pub const fn keyword(text: &str) -> Option<Self> {
        match text.as_bytes() {
            b"and" => Some(Self::And),
            b"break" => Some(Self::Break),
            b"do" => Some(Self::Do),
            b"else" => Some(Self::Else),
            b"elseif" => Some(Self::ElseIf),
            b"end" => Some(Self::End),
            b"false" => Some(Self::False),
            b"for" => Some(Self::For),
            b"function" => Some(Self::Function),
            b"global" => Some(Self::Global),
            b"goto" => Some(Self::Goto),
            b"if" => Some(Self::If),
            b"in" => Some(Self::In),
            b"local" => Some(Self::Local),
            b"nil" => Some(Self::Nil),
            b"not" => Some(Self::Not),
            b"or" => Some(Self::Or),
            b"repeat" => Some(Self::Repeat),
            b"return" => Some(Self::Return),
            b"then" => Some(Self::Then),
            b"true" => Some(Self::True),
            b"until" => Some(Self::Until),
            b"while" => Some(Self::While),
            _ => None,
        }
    }
}

/// A token with its original source text and byte span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token<'src> {
    kind: TokenKind,
    lexeme: &'src str,
    span: Span,
}

impl<'src> Token<'src> {
    /// Creates a token.
    #[must_use]
    pub const fn new(kind: TokenKind, lexeme: &'src str, span: Span) -> Self {
        Self { kind, lexeme, span }
    }

    /// Token kind.
    #[must_use]
    pub const fn kind(self) -> TokenKind {
        self.kind
    }

    /// Original source text for this token.
    #[must_use]
    pub const fn lexeme(self) -> &'src str {
        self.lexeme
    }

    /// Source span.
    #[must_use]
    pub const fn span(self) -> Span {
        self.span
    }
}
