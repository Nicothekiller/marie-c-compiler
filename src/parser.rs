use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{
    BinaryOp, Block, BlockItem, BuiltinType, ConstExpr, Declaration, Declarator, Expression,
    ExternalDeclaration, FunctionDeclaration, Parameter, Statement, TranslationUnit, Type, UnaryOp,
};
use crate::error::CompilerError;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
struct PestGeneratedParser;

#[derive(Default)]
pub struct CParser;

impl CParser {
    /// Creates a new parser frontend instance.
    pub fn new() -> Self {
        Self
    }

    /// Parses a preprocessed C translation unit and returns its AST representation.
    pub fn parse_translation_unit(&self, source: &str) -> Result<TranslationUnit, CompilerError> {
        let mut pairs = PestGeneratedParser::parse(Rule::translation_unit, source)
            .map_err(|error| CompilerError::Parse(error.to_string()))?;

        let Some(translation_unit_pair) = pairs.next() else {
            return Ok(TranslationUnit::default());
        };

        let mut top_level_items = Vec::new();
        for external_pair in translation_unit_pair.into_inner() {
            if external_pair.as_rule() != Rule::external_declaration {
                continue;
            }

            top_level_items.extend(parse_external_declaration(source, external_pair)?);
        }

        Ok(TranslationUnit { top_level_items })
    }
}

/// Lowers a parsed external declaration into one or more AST top-level items.
fn parse_external_declaration(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Vec<ExternalDeclaration>, CompilerError> {
    let Some(inner) = pair.into_inner().next() else {
        return Ok(Vec::new());
    };

    match inner.as_rule() {
        Rule::function_definition => Ok(vec![ExternalDeclaration::Function(
            parse_function_definition(source, inner)?,
        )]),
        Rule::declaration => {
            let declaration = parse_declaration(source, inner)?;
            Ok(vec![ExternalDeclaration::GlobalDeclaration(declaration)])
        }
        _ => Err(CompilerError::Parse(
            "unexpected external declaration".to_string(),
        )),
    }
}

/// Lowers a parsed function definition into a `FunctionDeclaration` AST node.
fn parse_function_definition(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<FunctionDeclaration, CompilerError> {
    let mut inner = pair.into_inner();

    let Some(specifier_pair) = inner.next() else {
        return Err(CompilerError::Parse(
            "missing function type specifier".to_string(),
        ));
    };
    let return_type = parse_declaration_specifiers(specifier_pair)?;

    let Some(declarator_pair) = inner.next() else {
        return Err(CompilerError::Parse(
            "missing function declarator".to_string(),
        ));
    };
    let (name, declarator_type) = parse_declarator(source, declarator_pair, return_type)?;

    let (return_type, params) = match declarator_type {
        Type::Function {
            return_type,
            params,
        } => (*return_type, params),
        _ => {
            return Err(CompilerError::Parse(
                "function definition must use function declarator".to_string(),
            ));
        }
    };

    let Some(body_pair) = inner.next() else {
        return Err(CompilerError::Parse("missing function body".to_string()));
    };
    let body = parse_compound_statement(source, body_pair)?;

    Ok(FunctionDeclaration {
        name,
        return_type,
        params,
        body: Some(body),
    })
}

/// Lowers a parsed declaration into a declaration AST node.
fn parse_declaration(source: &str, pair: Pair<'_, Rule>) -> Result<Declaration, CompilerError> {
    let mut inner = pair.into_inner();

    let Some(specifier_pair) = inner.next() else {
        return Ok(Declaration::default());
    };
    let base_type = parse_declaration_specifiers(specifier_pair)?;

    let mut declarators = Vec::new();
    for item in inner {
        if item.as_rule() != Rule::init_declarator_list {
            continue;
        }

        for declarator_pair in item.into_inner() {
            if declarator_pair.as_rule() != Rule::init_declarator {
                continue;
            }
            declarators.push(parse_init_declarator(
                source,
                declarator_pair,
                base_type.clone(),
            )?);
        }
    }

    Ok(Declaration { declarators })
}

/// Extracts the base type from declaration specifiers.
fn parse_declaration_specifiers(pair: Pair<'_, Rule>) -> Result<Type, CompilerError> {
    for item in pair.into_inner() {
        if item.as_rule() == Rule::type_specifier {
            return parse_type_specifier(item);
        }
    }

    Err(CompilerError::Parse("missing type specifier".to_string()))
}

/// Maps a parsed type-specifier rule to an AST `Type`.
fn parse_type_specifier(pair: Pair<'_, Rule>) -> Result<Type, CompilerError> {
    let Some(inner) = pair.into_inner().next() else {
        return Err(CompilerError::Parse("invalid type specifier".to_string()));
    };

    let ty = match inner.as_rule() {
        Rule::kw_int => BuiltinType::Int,
        Rule::kw_char => BuiltinType::Char,
        Rule::kw_void => BuiltinType::Void,
        _ => {
            return Err(CompilerError::Parse(
                "unsupported type specifier".to_string(),
            ));
        }
    };

    Ok(Type::Builtin(ty))
}

/// Lowers an init-declarator into a named declarator with optional initializer.
fn parse_init_declarator(
    source: &str,
    pair: Pair<'_, Rule>,
    base_type: Type,
) -> Result<Declarator, CompilerError> {
    let mut inner = pair.into_inner();

    let Some(declarator_pair) = inner.next() else {
        return Err(CompilerError::Parse("missing declarator".to_string()));
    };
    let (name, ty) = parse_declarator(source, declarator_pair, base_type)?;

    let initializer = inner
        .find(|item| item.as_rule() == Rule::assignment_expression)
        .map(|expr| parse_assignment_expression(source, expr))
        .transpose()?;

    Ok(Declarator {
        name,
        ty,
        initializer,
    })
}

/// Applies pointer and direct declarator suffixes to a base type.
fn parse_declarator(
    source: &str,
    pair: Pair<'_, Rule>,
    base_type: Type,
) -> Result<(String, Type), CompilerError> {
    let mut inner = pair.into_inner();

    let mut ty = base_type;
    let mut direct_declarator_pair = None;

    for item in inner.by_ref() {
        match item.as_rule() {
            Rule::pointer => {
                let pointer_depth = item.as_str().chars().filter(|ch| *ch == '*').count();
                for _ in 0..pointer_depth {
                    ty = Type::Pointer(Box::new(ty));
                }
            }
            Rule::direct_declarator => {
                direct_declarator_pair = Some(item);
                break;
            }
            _ => {}
        }
    }

    let Some(direct) = direct_declarator_pair else {
        return Err(CompilerError::Parse(
            "missing direct declarator".to_string(),
        ));
    };

    parse_direct_declarator(source, direct, ty)
}

/// Lowers a direct declarator into identifier name and composed type.
fn parse_direct_declarator(
    source: &str,
    pair: Pair<'_, Rule>,
    mut ty: Type,
) -> Result<(String, Type), CompilerError> {
    let mut inner = pair.into_inner();

    let Some(ident_pair) = inner.next() else {
        return Err(CompilerError::Parse("missing identifier".to_string()));
    };
    let name = ident_pair.as_str().to_string();

    for suffix in inner {
        if suffix.as_rule() != Rule::declarator_suffix {
            continue;
        }

        let Some(actual_suffix) = suffix.into_inner().next() else {
            continue;
        };

        match actual_suffix.as_rule() {
            Rule::array_suffix => {
                let size = actual_suffix
                    .into_inner()
                    .find(|item| item.as_rule() == Rule::assignment_expression)
                    .map(|size_expr| parse_assignment_expression(source, size_expr))
                    .transpose()?;

                let const_size = size.and_then(|expr| match expr {
                    Expression::IntegerLiteral(value) => Some(ConstExpr { value }),
                    _ => None,
                });

                ty = Type::Array {
                    element: Box::new(ty),
                    size: const_size,
                };
            }
            Rule::function_suffix => {
                let mut params = Vec::new();
                if let Some(param_list_pair) = actual_suffix.into_inner().next() {
                    params = parse_parameter_list(source, param_list_pair)?;
                }

                ty = Type::Function {
                    return_type: Box::new(ty),
                    params,
                };
            }
            _ => {}
        }
    }

    Ok((name, ty))
}

/// Parses a function parameter list into AST parameter entries.
fn parse_parameter_list(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Vec<Parameter>, CompilerError> {
    let mut params = Vec::new();

    for item in pair.into_inner() {
        if item.as_rule() != Rule::parameter_declaration {
            continue;
        }

        let mut inner = item.into_inner();

        let Some(specifier_pair) = inner.next() else {
            return Err(CompilerError::Parse("missing parameter type".to_string()));
        };
        let base_type = parse_declaration_specifiers(specifier_pair)?;

        let parameter = if let Some(declarator_pair) = inner.next() {
            let (name, ty) = parse_declarator(source, declarator_pair, base_type)?;
            Parameter {
                name: Some(name),
                ty,
            }
        } else {
            Parameter {
                name: None,
                ty: base_type,
            }
        };

        params.push(parameter);
    }

    Ok(params)
}

/// Lowers a compound statement into a block AST node.
fn parse_compound_statement(source: &str, pair: Pair<'_, Rule>) -> Result<Block, CompilerError> {
    let mut items = Vec::new();

    for item in pair.into_inner() {
        if item.as_rule() != Rule::block_item {
            continue;
        }

        let Some(content) = item.into_inner().next() else {
            continue;
        };

        match content.as_rule() {
            Rule::declaration => {
                items.push(BlockItem::Declaration(parse_declaration(source, content)?));
            }
            Rule::statement => {
                items.push(BlockItem::Statement(parse_statement(source, content)?));
            }
            _ => {}
        }
    }

    Ok(Block { items })
}

/// Lowers a generic statement rule into a specific `Statement` variant.
/// Lowers a generic statement rule into a specific `Statement` variant.
fn parse_statement(source: &str, pair: Pair<'_, Rule>) -> Result<Statement, CompilerError> {
    let Some(inner) = pair.into_inner().next() else {
        return Err(CompilerError::Parse("empty statement node".to_string()));
    };

    match inner.as_rule() {
        Rule::compound_statement => Ok(Statement::Block(parse_compound_statement(source, inner)?)),
        Rule::selection_statement => parse_selection_statement(source, inner),
        Rule::jump_statement => parse_jump_statement(source, inner),
        Rule::expression_statement => parse_expression_statement(source, inner),
        _ => Err(CompilerError::Parse("unsupported statement".to_string())),
    }
}

fn parse_selection_statement(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Statement, CompilerError> {
    let mut inner = pair.into_inner();

    let condition_pair = inner
        .find(|item| item.as_rule() == Rule::expression)
        .ok_or_else(|| CompilerError::Parse("if statement missing condition".to_string()))?;
    let condition = parse_expression(source, condition_pair)?;

    let statements: Vec<Pair<'_, Rule>> = inner
        .filter(|item| item.as_rule() == Rule::statement)
        .collect();
    if statements.is_empty() {
        return Err(CompilerError::Parse(
            "if statement missing branches".to_string(),
        ));
    }

    let then_branch = Box::new(parse_statement(source, statements[0].clone())?);
    let else_branch = if statements.len() > 1 {
        Some(Box::new(parse_statement(source, statements[1].clone())?))
    } else {
        None
    };

    Ok(Statement::If {
        condition,
        then_branch,
        else_branch,
    })
}

/// Lowers a jump statement (`return`) into AST.
fn parse_jump_statement(source: &str, pair: Pair<'_, Rule>) -> Result<Statement, CompilerError> {
    let expression = pair
        .into_inner()
        .find(|item| item.as_rule() == Rule::expression)
        .map(|expr| parse_expression(source, expr))
        .transpose()?;

    Ok(Statement::Return(expression))
}

/// Lowers an expression statement, including empty statements.
fn parse_expression_statement(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Statement, CompilerError> {
    let expression = pair
        .into_inner()
        .find(|item| item.as_rule() == Rule::expression)
        .map(|expr| parse_expression(source, expr))
        .transpose()?;

    Ok(Statement::Expression(expression))
}

/// Lowers an expression list and returns the last expression value.
fn parse_expression(source: &str, pair: Pair<'_, Rule>) -> Result<Expression, CompilerError> {
    let mut expressions = Vec::new();
    for item in pair.into_inner() {
        if item.as_rule() == Rule::assignment_expression {
            expressions.push(parse_assignment_expression(source, item)?);
        }
    }

    expressions
        .pop()
        .ok_or_else(|| CompilerError::Parse("empty expression".to_string()))
}

/// Lowers an assignment-expression, handling right-associative assignment.
fn parse_assignment_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    let inner: Vec<Pair<'_, Rule>> = pair.into_inner().collect();

    if inner.len() == 2
        && inner[0].as_rule() == Rule::unary_expression
        && inner[1].as_rule() == Rule::assignment_expression
    {
        let target = parse_unary_expression(source, inner[0].clone())?;
        let value = parse_assignment_expression(source, inner[1].clone())?;
        return Ok(Expression::Assignment {
            target: Box::new(target),
            value: Box::new(value),
        });
    }

    let Some(logical_or_pair) = inner
        .into_iter()
        .find(|item| item.as_rule() == Rule::logical_or_expression)
    else {
        return Err(CompilerError::Parse(
            "assignment expression missing logical_or_expression".to_string(),
        ));
    };

    parse_logical_or_expression(source, logical_or_pair)
}

/// Lowers a logical-or expression chain.
fn parse_logical_or_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::logical_and_expression,
        map_logical_or_operator,
    )
}

/// Lowers a logical-and expression chain.
fn parse_logical_and_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::equality_expression,
        map_logical_and_operator,
    )
}

/// Lowers an equality expression chain.
fn parse_equality_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::relational_expression,
        map_equality_operator,
    )
}

/// Lowers a relational expression chain.
fn parse_relational_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::additive_expression,
        map_relational_operator,
    )
}

/// Lowers an additive expression chain.
fn parse_additive_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::multiplicative_expression,
        map_additive_operator,
    )
}

/// Lowers a multiplicative expression chain.
fn parse_multiplicative_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::unary_expression,
        map_multiplicative_operator,
    )
}

/// Lowers a unary expression.
fn parse_unary_expression(source: &str, pair: Pair<'_, Rule>) -> Result<Expression, CompilerError> {
    let inner: Vec<Pair<'_, Rule>> = pair.into_inner().collect();

    if inner.len() == 2
        && inner[0].as_rule() == Rule::unary_operator
        && inner[1].as_rule() == Rule::unary_expression
    {
        let op = parse_unary_operator(inner[0].clone())?;
        let expr = parse_unary_expression(source, inner[1].clone())?;
        return Ok(Expression::Unary {
            op,
            expr: Box::new(expr),
        });
    }

    let Some(postfix_pair) = inner
        .into_iter()
        .find(|item| item.as_rule() == Rule::postfix_expression)
    else {
        return Err(CompilerError::Parse(
            "unary expression missing postfix expression".to_string(),
        ));
    };

    parse_postfix_expression(source, postfix_pair)
}

/// Maps unary operator token text to `UnaryOp`.
/// Maps unary operator token text to `UnaryOp`.
fn parse_unary_operator(pair: Pair<'_, Rule>) -> Result<UnaryOp, CompilerError> {
    match pair.as_str() {
        "&" => Ok(UnaryOp::AddressOf),
        "*" => Ok(UnaryOp::Dereference),
        "+" => Ok(UnaryOp::Plus),
        "-" => Ok(UnaryOp::Minus),
        "!" => Ok(UnaryOp::LogicalNot),
        _ => Err(CompilerError::Parse(
            "unsupported unary operator".to_string(),
        )),
    }
}

fn parse_postfix_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    let mut inner = pair.into_inner();

    let Some(primary_pair) = inner.next() else {
        return Err(CompilerError::Parse(
            "postfix expression missing primary expression".to_string(),
        ));
    };

    let mut expr = parse_primary_expression(source, primary_pair)?;

    for suffix in inner {
        if suffix.as_rule() != Rule::postfix_suffix {
            continue;
        }

        let Some(actual_suffix) = suffix.into_inner().next() else {
            continue;
        };

        match actual_suffix.as_rule() {
            Rule::expression => {
                let index = parse_expression(source, actual_suffix)?;
                expr = Expression::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                };
            }
            Rule::argument_expression_list => {
                let mut args = Vec::new();
                for item in actual_suffix.into_inner() {
                    if item.as_rule() == Rule::assignment_expression {
                        args.push(parse_assignment_expression(source, item)?);
                    }
                }
                expr = Expression::Call {
                    callee: Box::new(expr),
                    args,
                };
            }
            _ => {
                expr = Expression::Call {
                    callee: Box::new(expr),
                    args: Vec::new(),
                };
            }
        }
    }

    Ok(expr)
}

/// Lowers a primary expression (identifier, literal, or grouped expression).
fn parse_primary_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    let Some(inner) = pair.into_inner().next() else {
        return Err(CompilerError::Parse("empty primary expression".to_string()));
    };

    match inner.as_rule() {
        Rule::ident => Ok(Expression::Identifier(inner.as_str().to_string())),
        Rule::int_constant => {
            let value = inner
                .as_str()
                .parse::<i64>()
                .map_err(|error| CompilerError::Parse(error.to_string()))?;
            Ok(Expression::IntegerLiteral(value))
        }
        Rule::expression => parse_expression(source, inner),
        _ => Err(CompilerError::Parse(
            "unsupported primary expression".to_string(),
        )),
    }
}

/// Folds left-associative binary expressions for a precedence level.
/// Folds left-associative binary expressions for a precedence level.
fn fold_binary_by_rule(
    source: &str,
    pair: Pair<'_, Rule>,
    child_rule: Rule,
    operator_mapper: fn(&str) -> Result<BinaryOp, CompilerError>,
) -> Result<Expression, CompilerError> {
    let children: Vec<Pair<'_, Rule>> = pair
        .clone()
        .into_inner()
        .filter(|item| item.as_rule() == child_rule)
        .collect();

    if children.is_empty() {
        return Err(CompilerError::Parse(
            "expected binary child expression".to_string(),
        ));
    }

    let mut expression = parse_expression_by_rule(source, children[0].clone())?;
    for index in 1..children.len() {
        let op_text = between_trimmed(source, children[index - 1].clone(), children[index].clone());
        let op = operator_mapper(op_text.as_str())?;
        let rhs = parse_expression_by_rule(source, children[index].clone())?;
        expression = Expression::Binary {
            op,
            lhs: Box::new(expression),
            rhs: Box::new(rhs),
        };
    }

    Ok(expression)
}

/// Dispatches expression lowering by concrete grammar rule.
fn parse_expression_by_rule(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    match pair.as_rule() {
        Rule::logical_and_expression => parse_logical_and_expression(source, pair),
        Rule::equality_expression => parse_equality_expression(source, pair),
        Rule::relational_expression => parse_relational_expression(source, pair),
        Rule::additive_expression => parse_additive_expression(source, pair),
        Rule::multiplicative_expression => parse_multiplicative_expression(source, pair),
        Rule::unary_expression => parse_unary_expression(source, pair),
        _ => Err(CompilerError::Parse(
            "unsupported expression rule".to_string(),
        )),
    }
}

/// Returns the trimmed source text between two parsed spans.
/// Returns the trimmed source text between two parsed spans.
fn between_trimmed(source: &str, left: Pair<'_, Rule>, right: Pair<'_, Rule>) -> String {
    let left_end = left.as_span().end();
    let right_start = right.as_span().start();
    source[left_end..right_start].trim().to_string()
}

/// Maps a logical-or operator lexeme to `BinaryOp`.
fn map_logical_or_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("||") {
        Ok(BinaryOp::LogicalOr)
    } else {
        Err(CompilerError::Parse("expected || operator".to_string()))
    }
}

/// Maps a logical-and operator lexeme to `BinaryOp`.
fn map_logical_and_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("&&") {
        Ok(BinaryOp::LogicalAnd)
    } else {
        Err(CompilerError::Parse("expected && operator".to_string()))
    }
}

/// Maps an equality operator lexeme to `BinaryOp`.
fn map_equality_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("==") {
        Ok(BinaryOp::Equal)
    } else if text.contains("!=") {
        Ok(BinaryOp::NotEqual)
    } else {
        Err(CompilerError::Parse(
            "expected equality operator".to_string(),
        ))
    }
}

/// Maps a relational operator lexeme to `BinaryOp`.
fn map_relational_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("<=") {
        Ok(BinaryOp::LessEqual)
    } else if text.contains(">=") {
        Ok(BinaryOp::GreaterEqual)
    } else if text.contains('<') {
        Ok(BinaryOp::Less)
    } else if text.contains('>') {
        Ok(BinaryOp::Greater)
    } else {
        Err(CompilerError::Parse(
            "expected relational operator".to_string(),
        ))
    }
}

/// Maps an additive operator lexeme to `BinaryOp`.
fn map_additive_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains('+') {
        Ok(BinaryOp::Add)
    } else if text.contains('-') {
        Ok(BinaryOp::Subtract)
    } else {
        Err(CompilerError::Parse(
            "expected additive operator".to_string(),
        ))
    }
}

/// Maps a multiplicative operator lexeme to `BinaryOp`.
fn map_multiplicative_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains('*') {
        Ok(BinaryOp::Multiply)
    } else if text.contains('%') {
        Ok(BinaryOp::Modulo)
    } else {
        Err(CompilerError::Parse(
            "expected multiplicative operator".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        BinaryOp, BlockItem, BuiltinType, Expression, ExternalDeclaration, Statement, Type, UnaryOp,
    };

    use super::CParser;

    /// Parses source text into a translation unit for test assertions.
    fn parse_unit(source: &str) -> crate::ast::TranslationUnit {
        CParser::new()
            .parse_translation_unit(source)
            .expect("source should parse")
    }

    /// Asserts that source text fails parsing for unsupported or invalid grammar.
    fn assert_parse_fails(source: &str) {
        let result = CParser::new().parse_translation_unit(source);
        assert!(result.is_err(), "expected parse failure for: {source}");
    }

    /// Verifies lowering of an empty translation unit.
    #[test]
    fn lowers_empty_translation_unit() {
        let unit = parse_unit("");
        assert!(unit.top_level_items.is_empty());
    }

    /// Verifies lowering of a basic function definition.
    #[test]
    fn lowers_single_function_definition() {
        let unit = parse_unit("int main(void) { return 0; }");
        assert_eq!(unit.top_level_items.len(), 1);

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function external declaration");
        };

        assert_eq!(function.name, "main");
        assert_eq!(function.return_type, Type::Builtin(BuiltinType::Int));
        assert_eq!(function.params.len(), 1);
        assert!(matches!(
            function.params[0].ty,
            Type::Builtin(BuiltinType::Void)
        ));

        let body = function.body.as_ref().expect("function body should exist");
        assert_eq!(body.items.len(), 1);

        let BlockItem::Statement(Statement::Return(Some(Expression::IntegerLiteral(value)))) =
            &body.items[0]
        else {
            panic!("expected return integer literal statement");
        };

        assert_eq!(*value, 0);
    }

    /// Verifies lowering of a single global variable declaration.
    #[test]
    fn lowers_global_variable_declaration() {
        let unit = parse_unit("int counter;");
        assert_eq!(unit.top_level_items.len(), 1);

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected global declaration");
        };

        assert_eq!(declaration.declarators.len(), 1);
        let declarator = &declaration.declarators[0];
        assert_eq!(declarator.name, "counter");
        assert_eq!(declarator.ty, Type::Builtin(BuiltinType::Int));
        assert!(declarator.initializer.is_none());
    }

    /// Verifies pointer and fixed-size array declarator lowering.
    #[test]
    fn lowers_pointer_and_array_declarators() {
        let unit = parse_unit("char *ptr; int values[16];");
        assert_eq!(unit.top_level_items.len(), 2);

        let ExternalDeclaration::GlobalDeclaration(pointer_decl) = &unit.top_level_items[0] else {
            panic!("expected pointer declaration");
        };
        assert!(matches!(pointer_decl.declarators[0].ty, Type::Pointer(_)));

        let ExternalDeclaration::GlobalDeclaration(array_decl) = &unit.top_level_items[1] else {
            panic!("expected array declaration");
        };
        assert!(matches!(array_decl.declarators[0].ty, Type::Array { .. }));
    }

    /// Verifies lowering of an `if`/`else` statement shape.
    #[test]
    fn lowers_if_else_statement() {
        let unit = parse_unit("int main(void) { if (1) return 1; else return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::If {
            condition,
            then_branch,
            else_branch,
        }) = &body.items[0]
        else {
            panic!("expected if statement");
        };

        assert!(matches!(condition, Expression::IntegerLiteral(1)));
        assert!(matches!(**then_branch, Statement::Return(_)));
        assert!(else_branch.is_some());
        assert!(matches!(
            **else_branch.as_ref().expect("expected else branch"),
            Statement::Return(_)
        ));
    }

    /// Verifies additive vs multiplicative precedence in lowered AST.
    #[test]
    fn lowers_expression_precedence_for_add_and_multiply() {
        let unit = parse_unit("int main(void) { return 1 + 2 * 3; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Return(Some(Expression::Binary { op, rhs, .. }))) =
            &body.items[0]
        else {
            panic!("expected binary expression in return");
        };

        assert_eq!(*op, BinaryOp::Add);
        assert!(matches!(
            **rhs,
            Expression::Binary {
                op: BinaryOp::Multiply,
                ..
            }
        ));
    }

    /// Verifies nested unary operator lowering order.
    #[test]
    fn lowers_unary_expression_operators() {
        let unit = parse_unit("int main(void) { return !-x; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Return(Some(Expression::Unary { op, expr }))) =
            &body.items[0]
        else {
            panic!("expected unary expression in return");
        };

        assert_eq!(*op, UnaryOp::LogicalNot);
        assert!(matches!(
            **expr,
            Expression::Unary {
                op: UnaryOp::Minus,
                ..
            }
        ));
    }

    /// Verifies assignment expression lowering inside statements.
    #[test]
    fn lowers_assignment_statement() {
        let unit = parse_unit("int main(void) { x = 42; return x; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Expression(Some(Expression::Assignment { .. }))) =
            &body.items[0]
        else {
            panic!("expected assignment expression statement");
        };
    }

    /// Verifies lowering of call arguments containing index expressions.
    #[test]
    fn lowers_call_and_index_postfix_expressions() {
        let unit = parse_unit("int main(void) { return f(a[0], b); }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Return(Some(Expression::Call { args, .. }))) =
            &body.items[0]
        else {
            panic!("expected call expression in return");
        };

        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], Expression::Index { .. }));
    }

    /// Verifies lowering of a function prototype without body.
    #[test]
    fn lowers_function_prototype_as_function_item() {
        let unit = parse_unit("int sum(int a, int b);");
        assert_eq!(unit.top_level_items.len(), 1);

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected declaration external declaration");
        };

        assert_eq!(declaration.declarators.len(), 1);
        assert_eq!(declaration.declarators[0].name, "sum");
        assert!(matches!(
            declaration.declarators[0].ty,
            Type::Function { .. }
        ));
    }

    /// Verifies multiple declarators in a single declaration statement.
    #[test]
    fn lowers_multiple_declarators_in_one_declaration() {
        let unit = parse_unit("int a, b, c;");

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected declaration external declaration");
        };

        assert_eq!(declaration.declarators.len(), 3);
        assert_eq!(declaration.declarators[0].name, "a");
        assert_eq!(declaration.declarators[1].name, "b");
        assert_eq!(declaration.declarators[2].name, "c");
    }

    /// Verifies nested block statements are preserved in AST.
    #[test]
    fn lowers_nested_block_statement() {
        let unit = parse_unit("int main(void) { { return 1; } }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Block(inner_block)) = &body.items[0] else {
            panic!("expected inner block statement");
        };

        assert_eq!(inner_block.items.len(), 1);
        assert!(matches!(
            inner_block.items[0],
            BlockItem::Statement(Statement::Return(_))
        ));
    }

    /// Verifies `else if` lowers as an `else` branch containing another `if`.
    #[test]
    fn lowers_else_if_chain_shape() {
        let unit =
            parse_unit("int main(void) { if (a) return 1; else if (b) return 2; else return 3; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::If { else_branch, .. }) = &body.items[0] else {
            panic!("expected top-level if statement");
        };

        let Some(else_statement) = else_branch else {
            panic!("expected else branch");
        };

        assert!(matches!(**else_statement, Statement::If { .. }));
    }

    /// Verifies assignment is right-associative (`a = (b = 1)`).
    #[test]
    fn lowers_right_associative_assignment() {
        let unit = parse_unit("int main(void) { a = b = 1; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Expression(Some(Expression::Assignment {
            value, ..
        }))) = &body.items[0]
        else {
            panic!("expected outer assignment");
        };

        assert!(matches!(**value, Expression::Assignment { .. }));
    }

    /// Verifies relational/equality operator precedence (`<` before `==`).
    #[test]
    fn lowers_relational_before_equality() {
        let unit = parse_unit("int main(void) { return a < b == c; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Return(Some(Expression::Binary { op, lhs, .. }))) =
            &body.items[0]
        else {
            panic!("expected binary return expression");
        };

        assert_eq!(*op, BinaryOp::Equal);
        assert!(matches!(
            **lhs,
            Expression::Binary {
                op: BinaryOp::Less,
                ..
            }
        ));
    }

    /// Verifies logical-and has higher precedence than logical-or.
    #[test]
    fn lowers_logical_and_before_or() {
        let unit = parse_unit("int main(void) { return a || b && c; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Return(Some(Expression::Binary { op, rhs, .. }))) =
            &body.items[0]
        else {
            panic!("expected binary return expression");
        };

        assert_eq!(*op, BinaryOp::LogicalOr);
        assert!(matches!(
            **rhs,
            Expression::Binary {
                op: BinaryOp::LogicalAnd,
                ..
            }
        ));
    }

    /// Verifies declaration initializers are lowered as assignment expressions.
    #[test]
    fn lowers_declaration_initializer_expression() {
        let unit = parse_unit("int x = 5 + 7;");

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected declaration external declaration");
        };

        let Some(initializer) = &declaration.declarators[0].initializer else {
            panic!("expected initializer");
        };

        assert!(matches!(
            initializer,
            Expression::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    }

    /// Verifies empty statements inside a block lower correctly.
    #[test]
    fn lowers_empty_expression_statement() {
        let unit = parse_unit("int main(void) { ; return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = function.body.as_ref().expect("function body should exist");

        let BlockItem::Statement(Statement::Expression(None)) = &body.items[0] else {
            panic!("expected empty expression statement");
        };
    }

    /// Verifies division operator is rejected by current grammar.
    #[test]
    fn rejects_division_operator() {
        assert_parse_fails("int main(void) { return 10 / 2; }");
    }

    /// Verifies bitwise operators are rejected by current grammar.
    #[test]
    fn rejects_bitwise_operators() {
        assert_parse_fails("int main(void) { return a & b; }");
        assert_parse_fails("int main(void) { return a | b; }");
        assert_parse_fails("int main(void) { return a ^ b; }");
    }

    /// Verifies shift operators are rejected by current grammar.
    #[test]
    fn rejects_shift_operators() {
        assert_parse_fails("int main(void) { return a << 2; }");
        assert_parse_fails("int main(void) { return a >> 2; }");
    }

    /// Verifies loop statements outside v0 remain rejected.
    #[test]
    fn rejects_non_v0_loop_statements() {
        assert_parse_fails("int main(void) { while (1) return 0; }");
        assert_parse_fails("int main(void) { for (;;) return 0; }");
    }

    /// Verifies `static` storage class is not accepted in v0 grammar.
    #[test]
    fn rejects_static_storage_class() {
        assert_parse_fails("static int counter;");
    }

    /// Verifies malformed declarations fail parsing.
    #[test]
    fn rejects_malformed_declarations() {
        assert_parse_fails("int x[;");
        assert_parse_fails("int main(void) { int x = ; }");
    }

    /// Verifies malformed control-flow syntax fails parsing.
    #[test]
    fn rejects_malformed_if_syntax() {
        assert_parse_fails("int main(void) { if (1 return 0; }");
        assert_parse_fails("int main(void) { if 1) return 0; }");
    }
}
