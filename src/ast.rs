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
    /// Struct type (`struct Name { ... }` or `struct Name`).
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
}

/// Struct field declaration entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
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
    pub location: Option<crate::error::SourceLocation>,
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
    pub storage_class: Option<StorageClass>,
    pub declarators: Vec<Declarator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    Static,
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

/// Statement forms currently planned for 0.1.0/0.2.0.
#[derive(Debug, Clone)]
pub enum Statement {
    Block(Block),
    If {
        condition: Expression,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    While {
        condition: Expression,
        body: Box<Statement>,
    },
    For {
        init: Option<Expression>,
        condition: Option<Expression>,
        update: Option<Expression>,
        body: Box<Statement>,
    },
    Return(Option<Expression>),
    Expression(Option<Expression>),
    InlineAsm(Vec<String>),
}

/// Expression forms for the reduced C subset.
#[derive(Debug, Clone)]
pub enum Expression {
    /// Reference to a declared symbol by name.
    Identifier {
        name: String,
        location: Option<crate::error::SourceLocation>,
    },
    /// Integer literal constant.
    IntegerLiteral {
        value: i64,
        location: Option<crate::error::SourceLocation>,
    },
    /// Unary expression with one operand.
    Unary {
        /// Unary operator applied to the operand.
        op: UnaryOp,
        /// Operand expression.
        expr: Box<Expression>,
        location: Option<crate::error::SourceLocation>,
    },
    /// Binary expression with left and right operands.
    Binary {
        /// Binary operator joining both operands.
        op: BinaryOp,
        /// Left-hand side operand.
        lhs: Box<Expression>,
        /// Right-hand side operand.
        rhs: Box<Expression>,
        location: Option<crate::error::SourceLocation>,
    },
    /// Assignment expression (`target = value`).
    Assignment {
        /// Assignment target expression.
        target: Box<Expression>,
        /// Value expression assigned into `target`.
        value: Box<Expression>,
        location: Option<crate::error::SourceLocation>,
    },
    /// Function call expression (`callee(args...)`).
    Call {
        /// Function expression being invoked.
        callee: Box<Expression>,
        /// Call argument expressions in source order.
        args: Vec<Expression>,
        location: Option<crate::error::SourceLocation>,
    },
    /// Index expression (`base[index]`).
    Index {
        /// Base pointer/array expression.
        base: Box<Expression>,
        /// Index expression applied to `base`.
        index: Box<Expression>,
        location: Option<crate::error::SourceLocation>,
    },
    /// Array initializer (`{ expr, expr, ... }`).
    ArrayInitializer {
        elements: Vec<Expression>,
        location: Option<crate::error::SourceLocation>,
    },
    /// Struct member access (`base.member` or `base->member`).
    MemberAccess {
        base: Box<Expression>,
        member: String,
        through_pointer: bool,
        location: Option<crate::error::SourceLocation>,
    },
}

impl Expression {
    /// Returns source location associated with the expression, when available.
    pub fn location(&self) -> Option<crate::error::SourceLocation> {
        match self {
            Self::Identifier { location, .. }
            | Self::IntegerLiteral { location, .. }
            | Self::Unary { location, .. }
            | Self::Binary { location, .. }
            | Self::Assignment { location, .. }
            | Self::Call { location, .. }
            | Self::Index { location, .. }
            | Self::ArrayInitializer { location, .. }
            | Self::MemberAccess { location, .. } => *location,
        }
    }
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
    Divide,
    ShiftLeft,
    ShiftRight,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
}

impl TranslationUnit {
    /// Returns a human-readable s-expression representation of the AST.
    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        output.push_str("(TranslationUnit\n");

        for item in &self.top_level_items {
            write_external_declaration(&mut output, item, 1);
        }

        output.push_str(")\n");
        output
    }
}

fn write_external_declaration(output: &mut String, item: &ExternalDeclaration, depth: usize) {
    let indent = "  ".repeat(depth);

    match item {
        ExternalDeclaration::GlobalDeclaration(declaration) => {
            output.push_str(&format!("{indent}(GlobalDeclaration\n"));
            for declarator in &declaration.declarators {
                output.push_str(&format!(
                    "{indent}  (Declarator {} {:?}\n",
                    declarator.name, declarator.ty
                ));
                if let Some(initializer) = &declarator.initializer {
                    output.push_str(&format!("{indent}    (Initializer\n"));
                    write_expression(output, initializer, depth + 3);
                    output.push_str(&format!("{indent}    )\n"));
                }
                output.push_str(&format!("{indent}  )\n"));
            }
            output.push_str(&format!("{indent})\n"));
        }
        ExternalDeclaration::Function(function) => {
            output.push_str(&format!("{indent}(Function {}\n", function.name));
            output.push_str(&format!(
                "{indent}  (ReturnType {:?})\n",
                function.return_type
            ));
            output.push_str(&format!("{indent}  (Params\n"));
            for parameter in &function.params {
                output.push_str(&format!(
                    "{indent}    (Param {:?} {:?})\n",
                    parameter.name, parameter.ty
                ));
            }
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent}  (Body\n"));
            write_block(output, &function.body, depth + 2);
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent})\n"));
        }
    }
}

fn write_block(output: &mut String, block: &Block, depth: usize) {
    let indent = "  ".repeat(depth);
    output.push_str(&format!("{indent}(Block\n"));

    for item in &block.items {
        match item {
            BlockItem::Declaration(declaration) => {
                output.push_str(&format!("{indent}  (Declaration\n"));
                for declarator in &declaration.declarators {
                    output.push_str(&format!(
                        "{indent}    (Declarator {} {:?}\n",
                        declarator.name, declarator.ty
                    ));
                    if let Some(initializer) = &declarator.initializer {
                        output.push_str(&format!("{indent}      (Initializer\n"));
                        write_expression(output, initializer, depth + 4);
                        output.push_str(&format!("{indent}      )\n"));
                    }
                    output.push_str(&format!("{indent}    )\n"));
                }
                output.push_str(&format!("{indent}  )\n"));
            }
            BlockItem::Statement(statement) => write_statement(output, statement, depth + 1),
        }
    }

    output.push_str(&format!("{indent})\n"));
}

fn write_statement(output: &mut String, statement: &Statement, depth: usize) {
    let indent = "  ".repeat(depth);

    match statement {
        Statement::Block(block) => write_block(output, block, depth),
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            output.push_str(&format!("{indent}(If\n"));
            output.push_str(&format!("{indent}  (Condition\n"));
            write_expression(output, condition, depth + 2);
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent}  (Then\n"));
            write_statement(output, then_branch, depth + 2);
            output.push_str(&format!("{indent}  )\n"));
            if let Some(else_statement) = else_branch {
                output.push_str(&format!("{indent}  (Else\n"));
                write_statement(output, else_statement, depth + 2);
                output.push_str(&format!("{indent}  )\n"));
            }
            output.push_str(&format!("{indent})\n"));
        }
        Statement::Return(expression) => {
            output.push_str(&format!("{indent}(Return\n"));
            if let Some(expression) = expression {
                write_expression(output, expression, depth + 1);
            }
            output.push_str(&format!("{indent})\n"));
        }
        Statement::Expression(expression) => {
            output.push_str(&format!("{indent}(ExpressionStatement\n"));
            if let Some(expression) = expression {
                write_expression(output, expression, depth + 1);
            }
            output.push_str(&format!("{indent})\n"));
        }
        Statement::While { condition, body } => {
            output.push_str(&format!("{indent}(While\n"));
            output.push_str(&format!("{indent}  (Condition\n"));
            write_expression(output, condition, depth + 2);
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent}  (Body\n"));
            write_statement(output, body, depth + 2);
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent})\n"));
        }
        Statement::For {
            init,
            condition,
            update,
            body,
        } => {
            output.push_str(&format!("{indent}(For\n"));
            if let Some(init) = init {
                output.push_str(&format!("{indent}  (Init\n"));
                write_expression(output, init, depth + 2);
                output.push_str(&format!("{indent}  )\n"));
            }
            if let Some(condition) = condition {
                output.push_str(&format!("{indent}  (Condition\n"));
                write_expression(output, condition, depth + 2);
                output.push_str(&format!("{indent}  )\n"));
            }
            if let Some(update) = update {
                output.push_str(&format!("{indent}  (Update\n"));
                write_expression(output, update, depth + 2);
                output.push_str(&format!("{indent}  )\n"));
            }
            output.push_str(&format!("{indent}  (Body\n"));
            write_statement(output, body, depth + 2);
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent})\n"));
        }
        Statement::InlineAsm(instructions) => {
            output.push_str(&format!("{indent}(InlineAsm\n"));
            for instr in instructions {
                output.push_str(&format!("{indent}  {}\n", instr));
            }
            output.push_str(&format!("{indent})\n"));
        }
    }
}

fn write_expression(output: &mut String, expression: &Expression, depth: usize) {
    let indent = "  ".repeat(depth);

    match expression {
        Expression::Identifier { name, .. } => {
            output.push_str(&format!("{indent}(Identifier {name})\n"));
        }
        Expression::IntegerLiteral { value, .. } => {
            output.push_str(&format!("{indent}(IntegerLiteral {value})\n"));
        }
        Expression::Unary { op, expr, .. } => {
            output.push_str(&format!("{indent}(Unary {:?}\n", op));
            write_expression(output, expr, depth + 1);
            output.push_str(&format!("{indent})\n"));
        }
        Expression::Binary { op, lhs, rhs, .. } => {
            output.push_str(&format!("{indent}(Binary {:?}\n", op));
            write_expression(output, lhs, depth + 1);
            write_expression(output, rhs, depth + 1);
            output.push_str(&format!("{indent})\n"));
        }
        Expression::Assignment { target, value, .. } => {
            output.push_str(&format!("{indent}(Assignment\n"));
            write_expression(output, target, depth + 1);
            write_expression(output, value, depth + 1);
            output.push_str(&format!("{indent})\n"));
        }
        Expression::Call { callee, args, .. } => {
            output.push_str(&format!("{indent}(Call\n"));
            output.push_str(&format!("{indent}  (Callee\n"));
            write_expression(output, callee, depth + 2);
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent}  (Args\n"));
            for argument in args {
                write_expression(output, argument, depth + 2);
            }
            output.push_str(&format!("{indent}  )\n"));
            output.push_str(&format!("{indent})\n"));
        }
        Expression::Index { base, index, .. } => {
            output.push_str(&format!("{indent}(Index\n"));
            write_expression(output, base, depth + 1);
            write_expression(output, index, depth + 1);
            output.push_str(&format!("{indent})\n"));
        }
        Expression::ArrayInitializer { elements, .. } => {
            output.push_str(&format!("{indent}(ArrayInitializer\n"));
            for elem in elements {
                write_expression(output, elem, depth + 1);
            }
            output.push_str(&format!("{indent})\n"));
        }
        Expression::MemberAccess {
            base,
            member,
            through_pointer,
            ..
        } => {
            let access = if *through_pointer { "Arrow" } else { "Dot" };
            output.push_str(&format!("{indent}(MemberAccess {access} {member}\n"));
            write_expression(output, base, depth + 1);
            output.push_str(&format!("{indent})\n"));
        }
    }
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
                location: None,
            }],
            body: Block {
                items: vec![BlockItem::Statement(Statement::Return(Some(
                    Expression::IntegerLiteral {
                        value: 0,
                        location: None,
                    },
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
            lhs: Box::new(Expression::Identifier {
                name: "a".to_string(),
                location: None,
            }),
            rhs: Box::new(Expression::Identifier {
                name: "b".to_string(),
                location: None,
            }),
            location: None,
        };

        let Expression::Binary { op, .. } = expr else {
            panic!("expected binary expression");
        };

        assert_eq!(op, BinaryOp::Add);
    }

    #[test]
    fn pretty_print_contains_core_nodes() {
        let function = FunctionDeclaration {
            name: "main".to_string(),
            return_type: Type::Builtin(BuiltinType::Int),
            params: vec![Parameter {
                name: None,
                ty: Type::Builtin(BuiltinType::Void),
                location: None,
            }],
            body: Block {
                items: vec![BlockItem::Statement(Statement::Return(Some(
                    Expression::IntegerLiteral {
                        value: 0,
                        location: None,
                    },
                )))],
            },
        };

        let unit = TranslationUnit {
            top_level_items: vec![ExternalDeclaration::Function(function)],
        };

        let tree = unit.pretty_print();
        assert!(tree.contains("(TranslationUnit"));
        assert!(tree.contains("(Function main"));
        assert!(tree.contains("(Return"));
        assert!(tree.contains("(IntegerLiteral 0)"));
    }
}
