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
    /// Parser-produced string key for field syntax such as `table.name`.
    StringKey(&'src str),
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
    /// Table index expression.
    Index {
        /// Table expression.
        table: Box<Expr<'src>>,
        /// Key expression.
        key: Box<Expr<'src>>,
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

/// A block of Lua statements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block<'src> {
    statements: Vec<Stmt<'src>>,
    span: Span,
}

impl<'src> Block<'src> {
    /// Creates a statement block.
    #[must_use]
    pub const fn new(statements: Vec<Stmt<'src>>, span: Span) -> Self {
        Self { statements, span }
    }

    /// Statements in this block.
    #[must_use]
    pub fn statements(&self) -> &[Stmt<'src>] {
        &self.statements
    }

    /// Source span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// Lua statement node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stmt<'src> {
    kind: StmtKind<'src>,
    span: Span,
}

impl<'src> Stmt<'src> {
    /// Creates a statement node.
    #[must_use]
    pub const fn new(kind: StmtKind<'src>, span: Span) -> Self {
        Self { kind, span }
    }

    /// Statement kind.
    #[must_use]
    pub const fn kind(&self) -> &StmtKind<'src> {
        &self.kind
    }

    /// Source span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// Lua statement payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StmtKind<'src> {
    /// Empty statement, `;`.
    Empty,
    /// Assignment statement.
    Assign {
        /// Assignment targets.
        targets: Vec<Expr<'src>>,
        /// Assignment values.
        values: Vec<Expr<'src>>,
    },
    /// Function call statement.
    Call(Expr<'src>),
    /// Local declaration.
    Local {
        /// Declared names.
        names: Vec<NameDecl<'src>>,
        /// Initial values.
        values: Vec<Expr<'src>>,
    },
    /// Global declaration.
    Global(GlobalDecl<'src>),
    /// Function declaration.
    Function {
        /// Declaration kind.
        scope: FunctionScope,
        /// Function name.
        name: FunctionName<'src>,
        /// Function body.
        body: FunctionBody<'src>,
    },
    /// If statement.
    If {
        /// Condition/body clauses.
        clauses: Vec<IfClause<'src>>,
        /// Optional else block.
        else_block: Option<Block<'src>>,
    },
    /// While loop.
    While {
        /// Loop condition.
        condition: Expr<'src>,
        /// Loop body.
        body: Block<'src>,
    },
    /// Repeat-until loop.
    Repeat {
        /// Loop body.
        body: Block<'src>,
        /// Exit condition.
        condition: Expr<'src>,
    },
    /// Numeric for loop.
    NumericFor {
        /// Control variable.
        name: &'src str,
        /// Initial value.
        init: Expr<'src>,
        /// Limit value.
        limit: Expr<'src>,
        /// Optional step.
        step: Option<Expr<'src>>,
        /// Loop body.
        body: Block<'src>,
    },
    /// Generic for loop.
    GenericFor {
        /// Loop variable names.
        names: Vec<&'src str>,
        /// Iterator expression list.
        values: Vec<Expr<'src>>,
        /// Loop body.
        body: Block<'src>,
    },
    /// Return statement.
    Return(Vec<Expr<'src>>),
    /// Break statement.
    Break,
    /// Goto statement.
    Goto(&'src str),
    /// Label statement.
    Label(&'src str),
}

/// Local or global name declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameDecl<'src> {
    /// Name.
    pub name: &'src str,
    /// Optional declaration attribute.
    pub attribute: Option<&'src str>,
    /// Source span.
    pub span: Span,
}

/// Global declaration payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlobalDecl<'src> {
    /// Named global declarations.
    Names {
        /// Declared names.
        names: Vec<NameDecl<'src>>,
        /// Initial values.
        values: Vec<Expr<'src>>,
    },
    /// Collective `global *` declaration.
    All {
        /// Optional attribute.
        attribute: Option<&'src str>,
    },
}

/// Function declaration scope.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FunctionScope {
    /// Plain assignment-style function declaration.
    Plain,
    /// Local function declaration.
    Local,
    /// Global function declaration.
    Global,
}

/// Function name in a declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionName<'src> {
    /// Base name.
    pub base: &'src str,
    /// Dot-separated fields.
    pub fields: Vec<&'src str>,
    /// Optional method name after `:`.
    pub method: Option<&'src str>,
    /// Source span.
    pub span: Span,
}

/// Parsed function body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionBody<'src> {
    /// Parameters.
    pub params: Vec<Param<'src>>,
    /// Body block.
    pub block: Block<'src>,
    /// Source span.
    pub span: Span,
}

/// Function parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Param<'src> {
    /// Named parameter.
    Name(&'src str),
    /// Vararg parameter with optional Lua 5.5 vararg-table name.
    Vararg(Option<&'src str>),
}

/// If condition/body clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IfClause<'src> {
    /// Clause condition.
    pub condition: Expr<'src>,
    /// Clause body.
    pub block: Block<'src>,
}
