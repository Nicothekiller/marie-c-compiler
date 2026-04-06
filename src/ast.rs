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
    /// Function definition.
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

/// Function definition node.
#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub return_type: Type,
    pub params: Vec<Parameter>,
    pub body: Block,
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
    /// Reference to a declared symbol by name.
    Identifier(String),
    /// Integer literal constant.
    IntegerLiteral(i64),
    /// Unary expression with one operand.
    Unary {
        /// Unary operator applied to the operand.
        op: UnaryOp,
        /// Operand expression.
        expr: Box<Expression>,
    },
    /// Binary expression with left and right operands.
    Binary {
        /// Binary operator joining both operands.
        op: BinaryOp,
        /// Left-hand side operand.
        lhs: Box<Expression>,
        /// Right-hand side operand.
        rhs: Box<Expression>,
    },
    /// Assignment expression (`target = value`).
    Assignment {
        /// Assignment target expression.
        target: Box<Expression>,
        /// Value expression assigned into `target`.
        value: Box<Expression>,
    },
    /// Function call expression (`callee(args...)`).
    Call {
        /// Function expression being invoked.
        callee: Box<Expression>,
        /// Call argument expressions in source order.
        args: Vec<Expression>,
    },
    /// Index expression (`base[index]`).
    Index {
        /// Base pointer/array expression.
        base: Box<Expression>,
        /// Index expression applied to `base`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_unit_starts_empty() {
        let unit = TranslationUnit::default();
        assert!(unit.top_level_items.is_empty());
    }

    #[test]
    fn builds_function_definition_shape() {
        let function = FunctionDeclaration {
            name: "main".to_string(),
            return_type: Type::Builtin(BuiltinType::Int),
            params: vec![Parameter {
                name: Some("argc".to_string()),
                ty: Type::Builtin(BuiltinType::Int),
            }],
            body: Block {
                items: vec![BlockItem::Statement(Statement::Return(Some(
                    Expression::IntegerLiteral(0),
                )))],
            },
        };

        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(function.clone())],
        };

        assert_eq!(unit.top_level_items.len(), 1);
        let ExternalDeclaration::Function(found) = &unit.top_level_items[0] else {
            panic!("expected function external declaration");
        };

        assert_eq!(found.name, "main");
        assert_eq!(found.return_type, Type::Builtin(BuiltinType::Int));
        assert_eq!(found.params.len(), 1);
        assert_eq!(found.body.items.len(), 1);
    }

    #[test]
    fn supports_pointer_and_array_types() {
        let pointer = Type::Pointer(Box::new(Type::Builtin(BuiltinType::Char)));
        let array = Type::Array {
            element: Box::new(Type::Builtin(BuiltinType::Int)),
            size: Some(ConstExpr { value: 16 }),
        };

        assert!(matches!(pointer, Type::Pointer(_)));
        assert!(matches!(array, Type::Array { .. }));
    }

    #[test]
    fn builds_binary_expression_node() {
        let expr = Expression::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expression::Identifier("a".to_string())),
            rhs: Box::new(Expression::Identifier("b".to_string())),
        };

        let Expression::Binary { op, .. } = expr else {
            panic!("expected binary expression");
        };

        assert_eq!(op, BinaryOp::Add);
    }
}
