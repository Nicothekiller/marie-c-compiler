/// Root AST node representing a full C translation unit.
#[derive(Debug, Clone, Default)]
pub struct TranslationUnit {
    /// Top-level declarations and definitions in source order.
    pub top_level_items: Vec<ExternalDeclaration>,
}

/// Top-level declarations supported by the compiler frontend.
#[derive(Debug, Clone)]
pub enum ExternalDeclaration {
    /// Global variable declaration.
    GlobalDeclaration(Declaration),
    /// Function declaration or definition.
    Function(FunctionDeclaration),
}

/// Primitive builtin types supported in the current language subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinType {
    Int,
    Char,
    Void,
}

/// Type representation for declarations and expressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Builtin scalar type.
    Builtin(BuiltinType),
    /// Pointer type (`*T`).
    Pointer(Box<Type>),
    /// Fixed-size array type (`T[N]`).
    Array {
        element: Box<Type>,
        size: Option<ConstExpr>,
    },
    /// Function type (`T(params...)`).
    Function {
        return_type: Box<Type>,
        params: Vec<Parameter>,
    },
}

/// Compile-time integer expression placeholder for declarator sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstExpr {
    pub value: i64,
}

/// Named parameter in a function signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: Option<String>,
    pub ty: Type,
}

/// Variable declaration entry.
#[derive(Debug, Clone)]
pub struct Declarator {
    pub name: String,
    pub ty: Type,
    pub initializer: Option<Expression>,
}

/// Declaration statement/declaration-list node.
#[derive(Debug, Clone, Default)]
pub struct Declaration {
    pub declarators: Vec<Declarator>,
}

/// Function declaration or full definition.
#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub return_type: Type,
    pub params: Vec<Parameter>,
    pub body: Option<Block>,
}

/// Compound statement block with declarations/statements in source order.
#[derive(Debug, Clone, Default)]
pub struct Block {
    pub items: Vec<BlockItem>,
}

/// Item inside a compound statement.
#[derive(Debug, Clone)]
pub enum BlockItem {
    Declaration(Declaration),
    Statement(Statement),
}

/// Statement forms currently planned for v0/v1.
#[derive(Debug, Clone)]
pub enum Statement {
    Block(Block),
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    Return(Option<Expression>),
    Expression(Option<Expression>),
}

/// Expression forms for the reduced C subset.
#[derive(Debug, Clone)]
pub enum Expression {
    Identifier(String),
    IntegerLiteral(i64),
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    Assignment {
        target: Box<Expression>,
        value: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },
}

/// Unary operators supported by the parser subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    AddressOf,
    Dereference,
    Plus,
    Minus,
    LogicalNot,
}

/// Binary operators supported by the parser subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Multiply,
    Modulo,
    Add,
    Subtract,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    LogicalAnd,
    LogicalOr,
}
