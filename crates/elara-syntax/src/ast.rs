//! Syntax tree nodes.

use elara_core::Span;

/// Lua expression node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expr<'src> {
    kind: ExprKind<'src>,
    span: Span,
}

impl<'src> Expr<'src> {
    /// Creates an expression node.
    #[must_use]
    pub const fn new(kind: ExprKind<'src>, span: Span) -> Self {
        Self { kind, span }
    }

    /// Expression kind.
    #[must_use]
    pub const fn kind(&self) -> &ExprKind<'src> {
        &self.kind
    }

    /// Source span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// Lua expression payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExprKind<'src> {
    /// Syntax error placeholder.
    Error,
    /// `nil`.
    Nil,
    /// Boolean literal.
    Bool(bool),
    /// Integer numeral source text.
    Integer(&'src str),
    /// Floating-point numeral source text.
    Float(&'src str),
    /// String literal source text.
    String(&'src str),
    /// Name expression.
    Name(&'src str),
    /// Vararg expression, `...`.
    Vararg,
    /// Parenthesized expression.
    Grouped(Box<Expr<'src>>),
    /// Unary operator expression.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        expr: Box<Expr<'src>>,
    },
    /// Binary operator expression.
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<Expr<'src>>,
        /// Right operand.
        right: Box<Expr<'src>>,
    },
    /// Function or method call.
    Call {
        /// Callee expression.
        callee: Box<Expr<'src>>,
        /// Method name for `receiver:name(args)`.
        method: Option<&'src str>,
        /// Call arguments.
        args: Vec<Expr<'src>>,
    },
    /// Table constructor.
    Table(Vec<TableField<'src>>),
}

/// Lua unary operators.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnaryOp {
    /// `not`.
    Not,
    /// `#`.
    Len,
    /// Unary `-`.
    Neg,
    /// Unary `~`.
    BitNot,
}

/// Lua binary operators.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryOp {
    /// `or`.
    Or,
    /// `and`.
    And,
    /// `==`.
    Eq,
    /// `~=`.
    Ne,
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `>`.
    Gt,
    /// `>=`.
    Ge,
    /// `|`.
    BitOr,
    /// `~`.
    BitXor,
    /// `&`.
    BitAnd,
    /// `<<`.
    ShiftLeft,
    /// `>>`.
    ShiftRight,
    /// `..`.
    Concat,
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
    /// `//`.
    FloorDiv,
    /// `%`.
    Mod,
    /// `^`.
    Pow,
}

/// One field in a table constructor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableField<'src> {
    kind: TableFieldKind<'src>,
    span: Span,
}

impl<'src> TableField<'src> {
    /// Creates a table field.
    #[must_use]
    pub const fn new(kind: TableFieldKind<'src>, span: Span) -> Self {
        Self { kind, span }
    }

    /// Field kind.
    #[must_use]
    pub const fn kind(&self) -> &TableFieldKind<'src> {
        &self.kind
    }

    /// Source span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// Table constructor field payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TableFieldKind<'src> {
    /// Positional field.
    Array(Expr<'src>),
    /// `name = value` field.
    Named {
        /// Field name.
        name: &'src str,
        /// Field value.
        value: Expr<'src>,
    },
    /// `[key] = value` field.
    Keyed {
        /// Field key.
        key: Expr<'src>,
        /// Field value.
        value: Expr<'src>,
    },
}
