use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::{
    BinaryOp, Block, BlockItem, BuiltinType, ConstExpr, Declaration, Declarator, Expression,
    ExternalDeclaration, FunctionDeclaration, Parameter, Statement, StorageClass, StructField,
    TranslationUnit, Type, UnaryOp,
};
use crate::error::{CompilerError, SourceLocation};

fn pair_location(pair: &Pair<'_, Rule>) -> SourceLocation {
    let (line, column) = pair.as_span().start_pos().line_col();
    SourceLocation { line, column }
}

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
        let mut pairs =
            PestGeneratedParser::parse(Rule::translation_unit, source).map_err(|error| {
                let (line, column) = match error.line_col {
                    pest::error::LineColLocation::Pos((line, column)) => (line, column),
                    pest::error::LineColLocation::Span((line, column), _) => (line, column),
                };
                CompilerError::parse_at(
                    error.to_string(),
                    crate::error::SourceLocation { line, column },
                )
            })?;

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
            let (declaration, base_type) = parse_declaration_with_base(source, inner)?;
            if declaration.declarators.is_empty() && declaration.storage_class.is_none() {
                if let Type::Struct { .. } = base_type {
                    Ok(vec![ExternalDeclaration::TypeDeclaration(base_type)])
                } else {
                    Ok(vec![ExternalDeclaration::GlobalDeclaration(declaration)])
                }
            } else {
                Ok(vec![ExternalDeclaration::GlobalDeclaration(declaration)])
            }
        }
        _ => Err(CompilerError::parse(
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
        return Err(CompilerError::parse(
            "missing function type specifier".to_string(),
        ));
    };
    let (_static, return_type) = parse_declaration_specifiers(specifier_pair)?;

    let Some(declarator_pair) = inner.next() else {
        return Err(CompilerError::parse(
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
            return Err(CompilerError::parse(
                "function definition must use function declarator".to_string(),
            ));
        }
    };

    let Some(body_pair) = inner.next() else {
        return Err(CompilerError::parse("missing function body".to_string()));
    };
    let body = parse_compound_statement(source, body_pair)?;

    Ok(FunctionDeclaration {
        name,
        return_type,
        params,
        body,
    })
}

/// Lowers a parsed declaration into a declaration AST node.
fn parse_declaration(source: &str, pair: Pair<'_, Rule>) -> Result<Declaration, CompilerError> {
    let (declaration, _base_type) = parse_declaration_with_base(source, pair)?;
    Ok(declaration)
}

fn parse_declaration_with_base(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<(Declaration, Type), CompilerError> {
    let mut inner = pair.into_inner();

    let Some(specifier_pair) = inner.next() else {
        return Ok((Declaration::default(), Type::Builtin(BuiltinType::Int)));
    };
    let (storage_class, base_type) = parse_declaration_specifiers(specifier_pair)?;

    let mut declarators = Vec::new();
    for item in inner {
        if item.as_rule() != Rule::init_declarator_list {
            continue;
        }

        for declarator_pair in item.into_inner() {
            if declarator_pair.as_rule() != Rule::init_declarator {
                continue;
            }
            let declarator = parse_init_declarator(source, declarator_pair, base_type.clone())?;
            if matches!(declarator.ty, Type::Function { .. }) {
                return Err(CompilerError::parse(
                    "function prototypes are not supported".to_string(),
                ));
            }
            declarators.push(declarator);
        }
    }

    Ok((
        Declaration {
            storage_class,
            declarators,
        },
        base_type,
    ))
}

/// Extracts storage class and base type from declaration specifiers.
fn parse_declaration_specifiers(
    pair: Pair<'_, Rule>,
) -> Result<(Option<StorageClass>, Type), CompilerError> {
    let mut storage_class = None;
    let mut base_type = None;

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::storage_class_specifier => {
                storage_class = Some(parse_storage_class_specifier(item)?);
            }
            Rule::type_specifier => {
                base_type = Some(parse_type_specifier(item)?);
            }
            _ => {}
        }
    }

    let base_type =
        base_type.ok_or_else(|| CompilerError::parse("missing type specifier".to_string()))?;

    Ok((storage_class, base_type))
}

fn parse_storage_class_specifier(pair: Pair<'_, Rule>) -> Result<StorageClass, CompilerError> {
    let Some(inner) = pair.into_inner().next() else {
        return Err(CompilerError::parse("unknown storage class".to_string()));
    };

    if inner.as_rule() == Rule::kw_static {
        return Ok(StorageClass::Static);
    }
    if inner.as_rule() == Rule::kw_typedef {
        return Ok(StorageClass::Typedef);
    }
    Err(CompilerError::parse("unknown storage class".to_string()))
}

/// Maps a parsed type-specifier rule to an AST `Type`.
fn parse_type_specifier(pair: Pair<'_, Rule>) -> Result<Type, CompilerError> {
    let Some(inner) = pair.into_inner().next() else {
        return Err(CompilerError::parse("invalid type specifier".to_string()));
    };

    if inner.as_rule() == Rule::struct_specifier {
        return parse_struct_specifier(inner);
    }

    let builtin = match inner.as_rule() {
        Rule::kw_int => BuiltinType::Int,
        Rule::kw_char => BuiltinType::Char,
        Rule::kw_void => BuiltinType::Void,
        Rule::typedef_type => {
            return Ok(Type::Alias(inner.as_str().to_string()));
        }
        _ => {
            return Err(CompilerError::parse(
                "unsupported type specifier".to_string(),
            ));
        }
    };

    Ok(Type::Builtin(builtin))
}

fn parse_struct_specifier(pair: Pair<'_, Rule>) -> Result<Type, CompilerError> {
    let mut inner = pair.into_inner();

    let Some(first) = inner.next() else {
        return Err(CompilerError::parse("struct missing keyword".to_string()));
    };
    if first.as_rule() != Rule::kw_struct {
        return Err(CompilerError::parse("invalid struct specifier".to_string()));
    }

    let Some(name_pair) = inner.next() else {
        return Err(CompilerError::parse("struct missing name".to_string()));
    };

    let mut fields = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::struct_declaration {
            fields.extend(parse_struct_declaration(item)?);
        }
    }

    Ok(Type::Struct {
        name: name_pair.as_str().to_string(),
        fields,
    })
}

fn parse_struct_declaration(pair: Pair<'_, Rule>) -> Result<Vec<StructField>, CompilerError> {
    let source = pair.as_span().get_input();
    let mut inner = pair.into_inner();
    let Some(specifier_pair) = inner.next() else {
        return Err(CompilerError::parse(
            "struct field missing type specifier".to_string(),
        ));
    };
    let (_storage, base_type) = parse_declaration_specifiers(specifier_pair)?;

    let Some(declarator_list_pair) = inner.next() else {
        return Err(CompilerError::parse(
            "struct field missing declarator list".to_string(),
        ));
    };

    let mut fields = Vec::new();
    for item in declarator_list_pair.into_inner() {
        if item.as_rule() != Rule::struct_declarator {
            continue;
        }
        let Some(declarator_pair) = item.into_inner().next() else {
            continue;
        };
        let (name, ty) = parse_declarator(source, declarator_pair, base_type.clone())?;
        if matches!(ty, Type::Function { .. }) {
            return Err(CompilerError::parse(
                "function pointer fields are not supported".to_string(),
            ));
        }
        fields.push(StructField { name, ty });
    }

    Ok(fields)
}

/// Lowers an init-declarator into a named declarator with optional initializer.
fn parse_init_declarator(
    source: &str,
    pair: Pair<'_, Rule>,
    base_type: Type,
) -> Result<Declarator, CompilerError> {
    let mut inner = pair.into_inner();

    let Some(declarator_pair) = inner.next() else {
        return Err(CompilerError::parse("missing declarator".to_string()));
    };
    let (name, ty) = parse_declarator(source, declarator_pair, base_type)?;

    let initializer = inner
        .find_map(|item| {
            if item.as_rule() == Rule::initializer {
                for inner_item in item.into_inner() {
                    if inner_item.as_rule() == Rule::assignment_expression {
                        return Some(parse_assignment_expression(source, inner_item));
                    } else if inner_item.as_rule() == Rule::array_initializer {
                        return Some(parse_array_initializer(source, inner_item));
                    } else if inner_item.as_rule() == Rule::string_literal {
                        return Some(parse_string_literal_as_initializer(source, inner_item));
                    }
                }
                None
            } else if item.as_rule() == Rule::assignment_expression {
                Some(parse_assignment_expression(source, item))
            } else if item.as_rule() == Rule::array_initializer {
                Some(parse_array_initializer(source, item))
            } else if item.as_rule() == Rule::string_literal {
                Some(parse_string_literal_as_initializer(source, item))
            } else {
                None
            }
        })
        .transpose()?;

    Ok(Declarator {
        name,
        ty,
        initializer,
    })
}

fn parse_string_literal_as_initializer(
    _source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    let lit = pair.as_str();
    if lit.len() >= 2 && lit.starts_with('"') && lit.ends_with('"') {
        let content = &lit[1..lit.len() - 1];
        let unescaped = unescape_string_literal(content);

        let mut elements: Vec<Expression> = unescaped
            .chars()
            .map(|c| Expression::IntegerLiteral {
                value: c as i64,
                location: None,
            })
            .collect();

        elements.push(Expression::IntegerLiteral {
            value: 0,
            location: None,
        });

        Ok(Expression::ArrayInitializer {
            elements,
            location: Some(pair_location(&pair)),
        })
    } else {
        Err(CompilerError::parse("invalid string literal".to_string()))
    }
}

fn parse_array_initializer(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    let mut elements: Vec<Expression> = Vec::new();
    let location = pair_location(&pair);

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::assignment_expression => {
                elements.push(parse_assignment_expression(source, item)?);
            }
            _ => {}
        }
    }

    Ok(Expression::ArrayInitializer {
        elements,
        location: Some(location),
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
        return Err(CompilerError::parse(
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
        return Err(CompilerError::parse("missing identifier".to_string()));
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
                    Expression::IntegerLiteral { value, .. } => Some(ConstExpr { value }),
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
            return Err(CompilerError::parse("missing parameter type".to_string()));
        };
        let specifier_location = pair_location(&specifier_pair);
        let (_static, base_type) = parse_declaration_specifiers(specifier_pair)?;

        let parameter = if let Some(declarator_pair) = inner.next() {
            let declarator_location = pair_location(&declarator_pair);
            let (name, ty) = parse_declarator(source, declarator_pair, base_type)?;

            Parameter {
                name: Some(name),
                ty,
                location: Some(declarator_location),
            }
        } else {
            Parameter {
                name: None,
                ty: base_type,
                location: Some(specifier_location),
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
fn parse_statement(source: &str, pair: Pair<'_, Rule>) -> Result<Statement, CompilerError> {
    let Some(inner) = pair.into_inner().next() else {
        return Err(CompilerError::parse("empty statement node".to_string()));
    };

    match inner.as_rule() {
        Rule::compound_statement => Ok(Statement::Block(parse_compound_statement(source, inner)?)),
        Rule::selection_statement => parse_selection_statement(source, inner),
        Rule::iteration_statement => parse_iteration_statement(source, inner),
        Rule::jump_statement => parse_jump_statement(source, inner),
        Rule::expression_statement => parse_expression_statement(source, inner),
        Rule::inline_asm_statement => parse_inline_asm_statement(source, inner),
        _ => Err(CompilerError::parse("unsupported statement".to_string())),
    }
}

fn parse_inline_asm_statement(
    _source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Statement, CompilerError> {
    let location = pair_location(&pair);

    let mut inner = pair.into_inner();
    let _ = inner
        .next()
        .ok_or_else(|| CompilerError::parse_at("missing __asm", location))?;

    let asm_args_pair = inner
        .next()
        .ok_or_else(|| CompilerError::parse_at("missing asm arguments", location))?;

    Ok(Statement::InlineAsm(extract_asm_instructions(
        asm_args_pair,
    )?))
}

fn extract_asm_instructions(asm_args_pair: Pair<'_, Rule>) -> Result<Vec<String>, CompilerError> {
    let mut res: Vec<String> = vec![];

    for asm_pair in asm_args_pair.into_inner() {
        match asm_pair.as_rule() {
            Rule::string_literal => {
                let lit = asm_pair.as_str();
                if lit.len() >= 2 && lit.starts_with('"') && lit.ends_with('"') {
                    let content = &lit[1..lit.len() - 1];
                    res.push(unescape_string_literal(content));
                } else {
                    return Err(CompilerError::parse("invalid string literal"));
                }
            }
            Rule::asm_args => {
                res.extend(extract_asm_instructions(asm_pair)?);
            }
            _ => {}
        }
    }

    Ok(res)
}

fn unescape_string_literal(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn parse_selection_statement(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Statement, CompilerError> {
    let mut inner = pair.into_inner();

    let condition_pair = inner
        .find(|item| item.as_rule() == Rule::expression)
        .ok_or_else(|| CompilerError::parse("if statement missing condition".to_string()))?;
    let condition = parse_expression(source, condition_pair)?;

    let statements: Vec<Pair<'_, Rule>> = inner
        .filter(|item| item.as_rule() == Rule::statement)
        .collect();
    if statements.is_empty() {
        return Err(CompilerError::parse(
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

fn parse_iteration_statement(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Statement, CompilerError> {
    let inner: Vec<Pair<'_, Rule>> = pair.into_inner().collect();

    let first = &inner[0];
    if first.as_rule() == Rule::kw_while {
        let condition_pair = inner
            .iter()
            .find(|item| item.as_rule() == Rule::expression)
            .ok_or_else(|| CompilerError::parse("while missing condition".to_string()))?;
        let condition = parse_expression(source, condition_pair.clone())?;

        let body_pair = inner
            .iter()
            .find(|item| item.as_rule() == Rule::statement)
            .ok_or_else(|| CompilerError::parse("while missing body".to_string()))?;
        let body = Box::new(parse_statement(source, body_pair.clone())?);

        Ok(Statement::While { condition, body })
    } else {
        let expressions: Vec<Pair<'_, Rule>> = inner
            .iter()
            .filter(|item| item.as_rule() == Rule::expression)
            .cloned()
            .collect();

        let init = if !expressions.is_empty() {
            Some(parse_expression(source, expressions[0].clone())?)
        } else {
            None
        };

        let condition = if expressions.len() >= 2 {
            Some(parse_expression(source, expressions[1].clone())?)
        } else {
            None
        };

        let update = if expressions.len() >= 3 {
            Some(parse_expression(source, expressions[2].clone())?)
        } else {
            None
        };

        let body_pair = inner
            .iter()
            .find(|item| item.as_rule() == Rule::statement)
            .ok_or_else(|| CompilerError::parse("for missing body".to_string()))?;
        let body = Box::new(parse_statement(source, body_pair.clone())?);

        Ok(Statement::For {
            init,
            condition,
            update,
            body,
        })
    }
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
        .ok_or_else(|| CompilerError::parse("empty expression".to_string()))
}

/// Lowers an assignment-expression, handling right-associative assignment.
fn parse_assignment_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    let assignment_location = pair_location(&pair);
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
            location: Some(assignment_location),
        });
    }

    let Some(logical_or_pair) = inner
        .into_iter()
        .find(|item| item.as_rule() == Rule::logical_or_expression)
    else {
        return Err(CompilerError::parse(
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
        Rule::bitwise_or_expression,
        map_logical_and_operator,
    )
}

/// Lowers a bitwise-or expression chain.
fn parse_bitwise_or_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::bitwise_xor_expression,
        map_bitwise_or_operator,
    )
}

/// Lowers a bitwise-xor expression chain.
fn parse_bitwise_xor_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::bitwise_and_expression,
        map_bitwise_xor_operator,
    )
}

/// Lowers a bitwise-and expression chain.
fn parse_bitwise_and_expression(
    source: &str,
    pair: Pair<'_, Rule>,
) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(
        source,
        pair,
        Rule::equality_expression,
        map_bitwise_and_operator,
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
        Rule::shift_expression,
        map_relational_operator,
    )
}

/// Lowers a shift expression chain.
fn parse_shift_expression(source: &str, pair: Pair<'_, Rule>) -> Result<Expression, CompilerError> {
    fold_binary_by_rule(source, pair, Rule::additive_expression, map_shift_operator)
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
    let unary_location = pair_location(&pair);
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
            location: Some(unary_location),
        });
    }

    let Some(postfix_pair) = inner
        .into_iter()
        .find(|item| item.as_rule() == Rule::postfix_expression)
    else {
        return Err(CompilerError::parse(
            "unary expression missing postfix expression".to_string(),
        ));
    };

    parse_postfix_expression(source, postfix_pair)
}

/// Maps unary operator token text to `UnaryOp`.
fn parse_unary_operator(pair: Pair<'_, Rule>) -> Result<UnaryOp, CompilerError> {
    match pair.as_str() {
        "&" => Ok(UnaryOp::AddressOf),
        "*" => Ok(UnaryOp::Dereference),
        "+" => Ok(UnaryOp::Plus),
        "-" => Ok(UnaryOp::Minus),
        "!" => Ok(UnaryOp::LogicalNot),
        _ => Err(CompilerError::parse(
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
        return Err(CompilerError::parse(
            "postfix expression missing primary expression".to_string(),
        ));
    };

    let mut expr = parse_primary_expression(source, primary_pair)?;

    for suffix in inner {
        if suffix.as_rule() != Rule::postfix_suffix {
            continue;
        }

        let suffix_location = pair_location(&suffix);
        let Some(actual_suffix) = suffix.into_inner().next() else {
            continue;
        };

        match actual_suffix.as_rule() {
            Rule::index_suffix => {
                let Some(index_pair) = actual_suffix.into_inner().next() else {
                    return Err(CompilerError::parse(
                        "index suffix missing expression".to_string(),
                    ));
                };

                let index = parse_expression(source, index_pair)?;
                expr = Expression::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                    location: Some(suffix_location),
                };
            }
            Rule::call_suffix => {
                let mut args = Vec::new();
                if let Some(argument_list_pair) = actual_suffix.into_inner().next() {
                    for item in argument_list_pair.into_inner() {
                        if item.as_rule() == Rule::assignment_expression {
                            args.push(parse_assignment_expression(source, item)?);
                        }
                    }
                }

                expr = Expression::Call {
                    callee: Box::new(expr),
                    args,
                    location: Some(suffix_location),
                };
            }
            Rule::member_suffix | Rule::pointer_member_suffix => {
                let through_pointer = actual_suffix.as_rule() == Rule::pointer_member_suffix;
                let Some(member_pair) = actual_suffix.into_inner().next() else {
                    return Err(CompilerError::parse(
                        "member suffix missing field name".to_string(),
                    ));
                };

                expr = Expression::MemberAccess {
                    base: Box::new(expr),
                    member: member_pair.as_str().to_string(),
                    through_pointer,
                    location: Some(suffix_location),
                };
            }
            _ => {}
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
        return Err(CompilerError::parse("empty primary expression".to_string()));
    };

    match inner.as_rule() {
        Rule::ident => Ok(Expression::Identifier {
            name: inner.as_str().to_string(),
            location: Some(pair_location(&inner)),
        }),
        Rule::int_constant => {
            let value = inner
                .as_str()
                .parse::<i64>()
                .map_err(|error| CompilerError::parse(error.to_string()))?;
            Ok(Expression::IntegerLiteral {
                value,
                location: Some(pair_location(&inner)),
            })
        }
        Rule::expression => parse_expression(source, inner),
        _ => Err(CompilerError::parse(
            "unsupported primary expression".to_string(),
        )),
    }
}

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
        return Err(CompilerError::parse(
            "expected binary child expression".to_string(),
        ));
    }

    let binary_location = pair_location(&pair);
    let mut expression = parse_expression_by_rule(source, children[0].clone())?;
    for index in 1..children.len() {
        let op_text = between_trimmed(source, children[index - 1].clone(), children[index].clone());
        let op = operator_mapper(op_text.as_str())?;
        let rhs = parse_expression_by_rule(source, children[index].clone())?;
        expression = Expression::Binary {
            op,
            lhs: Box::new(expression),
            rhs: Box::new(rhs),
            location: Some(binary_location),
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
        Rule::logical_or_expression => parse_logical_or_expression(source, pair),
        Rule::logical_and_expression => parse_logical_and_expression(source, pair),
        Rule::bitwise_or_expression => parse_bitwise_or_expression(source, pair),
        Rule::bitwise_xor_expression => parse_bitwise_xor_expression(source, pair),
        Rule::bitwise_and_expression => parse_bitwise_and_expression(source, pair),
        Rule::equality_expression => parse_equality_expression(source, pair),
        Rule::relational_expression => parse_relational_expression(source, pair),
        Rule::shift_expression => parse_shift_expression(source, pair),
        Rule::additive_expression => parse_additive_expression(source, pair),
        Rule::multiplicative_expression => parse_multiplicative_expression(source, pair),
        Rule::unary_expression => parse_unary_expression(source, pair),
        _ => Err(CompilerError::parse(
            "unsupported expression rule".to_string(),
        )),
    }
}

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
        Err(CompilerError::parse("expected || operator".to_string()))
    }
}

/// Maps a logical-and operator lexeme to `BinaryOp`.
fn map_logical_and_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("&&") {
        Ok(BinaryOp::LogicalAnd)
    } else {
        Err(CompilerError::parse("expected && operator".to_string()))
    }
}

/// Maps a bitwise-or operator lexeme to `BinaryOp`.
fn map_bitwise_or_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains('|') {
        Ok(BinaryOp::BitwiseOr)
    } else {
        Err(CompilerError::parse("expected | operator".to_string()))
    }
}

/// Maps a bitwise-xor operator lexeme to `BinaryOp`.
fn map_bitwise_xor_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains('^') {
        Ok(BinaryOp::BitwiseXor)
    } else {
        Err(CompilerError::parse("expected ^ operator".to_string()))
    }
}

/// Maps a bitwise-and operator lexeme to `BinaryOp`.
fn map_bitwise_and_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("&&") {
        Ok(BinaryOp::LogicalAnd)
    } else if text.contains('&') {
        Ok(BinaryOp::BitwiseAnd)
    } else {
        Err(CompilerError::parse("expected & operator".to_string()))
    }
}

/// Maps a shift operator lexeme to `BinaryOp`.
fn map_shift_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("<<") {
        Ok(BinaryOp::ShiftLeft)
    } else if text.contains(">>") {
        Ok(BinaryOp::ShiftRight)
    } else {
        Err(CompilerError::parse(
            "expected << or >> operator".to_string(),
        ))
    }
}

/// Maps an equality operator lexeme to `BinaryOp`.
fn map_equality_operator(text: &str) -> Result<BinaryOp, CompilerError> {
    if text.contains("==") {
        Ok(BinaryOp::Equal)
    } else if text.contains("!=") {
        Ok(BinaryOp::NotEqual)
    } else {
        Err(CompilerError::parse(
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
        Err(CompilerError::parse(
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
        Err(CompilerError::parse(
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
    } else if text.contains('/') {
        Ok(BinaryOp::Divide)
    } else {
        Err(CompilerError::parse(
            "expected multiplicative operator".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        BinaryOp, BlockItem, BuiltinType, Expression, ExternalDeclaration, Statement, StorageClass,
        Type, UnaryOp,
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

    /// Asserts that source text parses successfully.
    fn assert_parse_succeeds(source: &str) {
        let result = CParser::new().parse_translation_unit(source);
        assert!(result.is_ok(), "expected parse success for: {source}");
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

        let body = &function.body;
        assert_eq!(body.items.len(), 1);

        let BlockItem::Statement(Statement::Return(Some(Expression::IntegerLiteral {
            value, ..
        }))) = &body.items[0]
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
        let body = &function.body;

        let BlockItem::Statement(Statement::If {
            condition,
            then_branch,
            else_branch,
        }) = &body.items[0]
        else {
            panic!("expected if statement");
        };

        assert!(matches!(
            condition,
            Expression::IntegerLiteral { value: 1, .. }
        ));
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
        let body = &function.body;

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
        let body = &function.body;

        let BlockItem::Statement(Statement::Return(Some(Expression::Unary { op, expr, .. }))) =
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
        let body = &function.body;

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
        let body = &function.body;

        let BlockItem::Statement(Statement::Return(Some(Expression::Call { args, .. }))) =
            &body.items[0]
        else {
            panic!("expected call expression in return");
        };

        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], Expression::Index { .. }));
    }

    /// Verifies function prototype syntax is rejected by parser.
    #[test]
    fn rejects_function_prototype_syntax() {
        assert_parse_fails("int sum(int a, int b);");
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
        let body = &function.body;

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
        let body = &function.body;

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
        let body = &function.body;

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
        let body = &function.body;

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
        let unit = parse_unit("int main(void) { return 1 || 0 && 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::Return(Some(Expression::Binary {
            op, lhs, rhs, ..
        }))) = &body.items[0]
        else {
            panic!("expected binary return expression");
        };

        assert_eq!(*op, BinaryOp::LogicalOr, "top-level should be LogicalOr");
        let lhs_inner = &**lhs;
        assert!(
            matches!(lhs_inner, Expression::IntegerLiteral { value: 1, .. }),
            "lhs should be 1"
        );
        let rhs_inner = &**rhs;
        assert!(
            matches!(rhs_inner, Expression::Binary { .. }),
            "rhs should be a Binary expression (the && part)"
        );
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
        let body = &function.body;

        let BlockItem::Statement(Statement::Expression(None)) = &body.items[0] else {
            panic!("expected empty expression statement");
        };
    }

    /// Verifies division operator is accepted by parser (rejected by target validation).
    #[test]
    fn parses_division_operator() {
        assert_parse_succeeds("int main(void) { return 10 / 2; }");
    }

    /// Verifies division with variables parse.
    #[test]
    fn parses_division_variables() {
        assert_parse_succeeds("int main(void) { return a / b; }");
    }

    /// Verifies division with expressions parse.
    #[test]
    fn parses_division_expressions() {
        assert_parse_succeeds("int main(void) { return (a + b) / c; }");
    }

    /// Verifies bitwise operators parse (caught by target validation).
    #[test]
    fn parses_bitwise_operators() {
        assert_parse_succeeds("int main(void) { return a & b; }");
        assert_parse_succeeds("int main(void) { return a | b; }");
        assert_parse_succeeds("int main(void) { return a ^ b; }");
    }

    /// Verifies bitwise operators with compound expressions parse.
    #[test]
    fn parses_bitwise_operators_compound() {
        assert_parse_succeeds("int main(void) { return (a & b) | c; }");
        assert_parse_succeeds("int main(void) { return a ^ (b & c); }");
    }

    /// Verifies bitwise operators with other operators parse.
    #[test]
    fn parses_bitwise_operators_mixed() {
        assert_parse_succeeds("int main(void) { return a + b & c; }");
        assert_parse_succeeds("int main(void) { return a | b == c; }");
    }

    /// Verifies shift operators parse (caught by target validation).
    #[test]
    fn parses_shift_operators() {
        assert_parse_succeeds("int main(void) { return a << 2; }");
        assert_parse_succeeds("int main(void) { return a >> 2; }");
    }

    /// Verifies shift operators with constants parse.
    #[test]
    fn parses_shift_operators_constants() {
        assert_parse_succeeds("int main(void) { return 1 << 3; }");
        assert_parse_succeeds("int main(void) { return 8 >> 2; }");
    }

    /// Verifies shift operators with expressions parse.
    #[test]
    fn parses_shift_operators_expressions() {
        assert_parse_succeeds("int main(void) { return (a + b) << c; }");
        assert_parse_succeeds("int main(void) { return a >> (b + 1); }");
    }

    /// Verifies loop statements now parse (caught by target validation).
    #[test]
    fn parses_loop_statements() {
        assert_parse_succeeds("int main(void) { while (1) return 0; }");
        assert_parse_succeeds("int main(void) { for (;;) return 0; }");
    }

    /// Verifies while loop with condition parse.
    #[test]
    fn parses_while_loop_with_condition() {
        assert_parse_succeeds("int main(void) { while (x < 10) x = x + 1; }");
        assert_parse_succeeds("int main(void) { while (1) { } }");
    }

    /// Verifies for loop with all parts parse.
    #[test]
    fn parses_for_loop_parts() {
        assert_parse_succeeds("int main(void) { for (i = 0; i < 10; i = i + 1) { } }");
        assert_parse_succeeds("int main(void) { for (;;) break; }");
    }

    /// Verifies `static` storage class parses but is caught by target validation.
    #[test]
    fn parses_static_storage_class() {
        assert_parse_succeeds("static int counter;");
    }

    /// Verifies static with initialization parses.
    #[test]
    fn parses_static_with_initializer() {
        assert_parse_succeeds("static int x = 5;");
        assert_parse_succeeds("static int arr[3];");
    }

    /// Verifies static in function parse.
    #[test]
    fn parses_static_in_function() {
        assert_parse_succeeds("int main(void) { static int x; return x; }");
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

    /// Verifies `void`-only parameter list remains accepted.
    #[test]
    fn accepts_void_parameter_marker() {
        let unit = parse_unit("int main(void) { return 0; }");
        assert_eq!(unit.top_level_items.len(), 1);
    }

    /// Verifies missing semicolon is rejected.
    #[test]
    fn rejects_missing_semicolon_after_return() {
        assert_parse_fails("int main(void) { return 1 }");
    }

    /// Verifies inline asm statement parses and extracts instructions.
    #[test]
    fn parses_inline_asm_statement() {
        let unit = parse_unit("int main(void) { __asm(\"Load x\"); return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::InlineAsm(instructions)) = &body.items[0] else {
            panic!("expected inline asm statement");
        };

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0], "Load x");
    }

    /// Verifies inline asm with multiple string arguments.
    #[test]
    fn parses_inline_asm_multiple_strings() {
        let unit = parse_unit("int main(void) { __asm(\"Load x\", \"Add y\"); return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::InlineAsm(instructions)) = &body.items[0] else {
            panic!("expected inline asm statement");
        };

        assert_eq!(instructions.len(), 2);
        assert_eq!(instructions[0], "Load x");
        assert_eq!(instructions[1], "Add y");
    }

    /// Verifies inline asm escape sequences are unescaped.
    #[test]
    fn parses_inline_asm_escape_sequences() {
        let unit = parse_unit("int main(void) { __asm(\"Line1\\nLine2\"); return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::InlineAsm(instructions)) = &body.items[0] else {
            panic!("expected inline asm statement");
        };

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0], "Line1\nLine2");
    }

    /// Verifies inline asm with tab escape sequence.
    #[test]
    fn parses_inline_asm_tab_escape() {
        let unit = parse_unit("int main(void) { __asm(\"Col1\\tCol2\"); return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::InlineAsm(instructions)) = &body.items[0] else {
            panic!("expected inline asm statement");
        };

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0], "Col1\tCol2");
    }

    /// Verifies inline asm with backslash escape.
    #[test]
    fn parses_inline_asm_backslash_escape() {
        let unit = parse_unit("int main(void) { __asm(\"path\\\\to\\\\file\"); return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::InlineAsm(instructions)) = &body.items[0] else {
            panic!("expected inline asm statement");
        };

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0], "path\\to\\file");
    }

    /// Verifies inline asm with quote escape.
    #[test]
    fn parses_inline_asm_quote_escape() {
        let unit = parse_unit("int main(void) { __asm(\"say \\\"hello\\\"\"); return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::InlineAsm(instructions)) = &body.items[0] else {
            panic!("expected inline asm statement");
        };

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0], "say \"hello\"");
    }

    /// Verifies malformed inline asm is rejected - missing closing paren.
    #[test]
    fn rejects_inline_asm_missing_paren() {
        assert_parse_fails("int main(void) { __asm(\"Load x\"; return 0; }");
    }

    /// Verifies inline asm without semicolon is rejected.
    #[test]
    fn rejects_inline_asm_no_semicolon() {
        assert_parse_fails("int main(void) { __asm(\"Load x\") return 0; }");
    }

    /// Verifies inline asm with empty string parses.
    #[test]
    fn parses_inline_asm_empty_string() {
        let unit = parse_unit("int main(void) { __asm(\"\"); return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };
        let body = &function.body;

        let BlockItem::Statement(Statement::InlineAsm(instructions)) = &body.items[0] else {
            panic!("expected inline asm statement");
        };

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0], "");
    }

    /// Verifies array initializer parses with multiple elements.
    #[test]
    fn parses_array_initializer_multiple_elements() {
        let unit = parse_unit("int arr[3] = { 1, 2, 3 };");

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected global declaration");
        };

        let Some(initializer) = &declaration.declarators[0].initializer else {
            panic!("expected initializer");
        };

        assert!(matches!(
            initializer,
            Expression::ArrayInitializer { elements, .. } if elements.len() == 3
        ));
    }

    /// Verifies array initializer with single element parses.
    #[test]
    fn parses_array_initializer_single_element() {
        let unit = parse_unit("int arr[1] = { 42 };");

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected global declaration");
        };

        let Some(initializer) = &declaration.declarators[0].initializer else {
            panic!("expected initializer");
        };

        assert!(matches!(
            initializer,
            Expression::ArrayInitializer { elements, .. } if elements.len() == 1
        ));
    }

    /// Verifies array initializer in local variable declaration.
    #[test]
    fn parses_array_initializer_local_variable() {
        let unit = parse_unit("int main(void) { int arr[2] = { 5, 10 }; return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };

        let BlockItem::Declaration(declaration) = &function.body.items[0] else {
            panic!("expected declaration");
        };

        let Some(initializer) = &declaration.declarators[0].initializer else {
            panic!("expected initializer");
        };

        assert!(matches!(initializer, Expression::ArrayInitializer { .. }));
    }

    /// Verifies regular scalar initializer still works.
    #[test]
    fn parses_scalar_initializer_still_works() {
        let unit = parse_unit("int x = 42;");

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected global declaration");
        };

        let Some(initializer) = &declaration.declarators[0].initializer else {
            panic!("expected initializer");
        };

        assert!(matches!(
            initializer,
            Expression::IntegerLiteral { value: 42, .. }
        ));
    }

    /// Verifies string literal initializer parses as array of char codes.
    #[test]
    fn parses_string_literal_initializer() {
        let unit = parse_unit("char str[6] = \"hello\";");

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected global declaration");
        };

        let declarator = &declaration.declarators[0];
        assert_eq!(declarator.name, "str");

        let Some(initializer) = &declarator.initializer else {
            panic!("expected initializer");
        };

        let Expression::ArrayInitializer { elements, .. } = initializer else {
            panic!("expected array initializer");
        };

        assert_eq!(elements.len(), 6);

        if let Expression::IntegerLiteral { value, .. } = &elements[0] {
            assert_eq!(*value, 'h' as i64);
        } else {
            panic!("expected first element to be 'h'");
        }

        if let Expression::IntegerLiteral { value, .. } = &elements[4] {
            assert_eq!(*value, 'o' as i64);
        } else {
            panic!("expected fifth element to be 'o'");
        }

        if let Expression::IntegerLiteral { value, .. } = &elements[5] {
            assert_eq!(*value, 0);
        } else {
            panic!("expected sixth element to be null terminator");
        }
    }

    /// Verifies string literal initializer in local variable.
    #[test]
    fn parses_string_literal_initializer_local() {
        let unit = parse_unit("int main(void) { char msg[5] = \"test\"; return 0; }");

        let ExternalDeclaration::Function(function) = &unit.top_level_items[0] else {
            panic!("expected function declaration");
        };

        let BlockItem::Declaration(declaration) = &function.body.items[0] else {
            panic!("expected declaration");
        };

        let Some(initializer) = &declaration.declarators[0].initializer else {
            panic!("expected initializer");
        };

        assert!(
            matches!(initializer, Expression::ArrayInitializer { elements, .. } if elements.len() == 5)
        );
    }

    #[test]
    fn parses_struct_declaration_and_member_access() {
        let unit = parse_unit("struct Point { int x; int y; } p; int main(void) { return p.x; }");

        let ExternalDeclaration::GlobalDeclaration(declaration) = &unit.top_level_items[0] else {
            panic!("expected global declaration");
        };

        let ty = &declaration.declarators[0].ty;
        assert!(
            matches!(ty, Type::Struct { name, fields } if name == "Point" && fields.len() == 2)
        );

        let ExternalDeclaration::Function(function) = &unit.top_level_items[1] else {
            panic!("expected function declaration");
        };

        let BlockItem::Statement(Statement::Return(Some(Expression::MemberAccess {
            member,
            through_pointer,
            ..
        }))) = &function.body.items[0]
        else {
            panic!("expected member access");
        };

        assert_eq!(member, "x");
        assert!(!through_pointer);
    }

    #[test]
    fn parses_pointer_member_access_arrow() {
        let unit = parse_unit(
            "struct Point { int x; int y; } p; int main(void) { struct Point *q; q = &p; return q->y; }",
        );

        let ExternalDeclaration::Function(function) = &unit.top_level_items[1] else {
            panic!("expected function declaration");
        };

        let BlockItem::Statement(Statement::Return(Some(Expression::MemberAccess {
            member,
            through_pointer,
            ..
        }))) = &function.body.items[2]
        else {
            panic!("expected member access");
        };

        assert_eq!(member, "y");
        assert!(*through_pointer);
    }

    #[test]
    fn parses_tag_only_struct_declaration() {
        let unit = parse_unit("struct Point { int x; int y; };");
        assert_eq!(unit.top_level_items.len(), 1);

        let ExternalDeclaration::TypeDeclaration(ty) = &unit.top_level_items[0] else {
            panic!("expected type declaration");
        };

        assert!(
            matches!(ty, Type::Struct { name, fields } if name == "Point" && fields.len() == 2)
        );
    }

    #[test]
    fn parses_typedef_struct_alias_usage() {
        let unit = parse_unit(
            "typedef struct Point { int x; int y; } Point; Point p; int main(void) { return p.x; }",
        );

        let ExternalDeclaration::GlobalDeclaration(td) = &unit.top_level_items[0] else {
            panic!("expected typedef declaration");
        };
        assert!(matches!(td.storage_class, Some(StorageClass::Typedef)));

        let ExternalDeclaration::GlobalDeclaration(var) = &unit.top_level_items[1] else {
            panic!("expected variable declaration");
        };
        assert!(matches!(var.declarators[0].ty, Type::Alias(ref name) if name == "Point"));
    }

    #[test]
    fn parses_struct_tag_forward_reference() {
        let unit = parse_unit("struct Point; struct Point { int x; }; struct Point p;");
        assert_eq!(unit.top_level_items.len(), 3);
    }

    #[test]
    fn parses_pointer_to_typedef_struct() {
        let unit = parse_unit("typedef struct Point { int x; } Point; Point *p;");
        let ExternalDeclaration::GlobalDeclaration(var) = &unit.top_level_items[1] else {
            panic!("expected variable declaration");
        };
        assert!(matches!(var.declarators[0].ty, Type::Pointer(_)));
    }

    #[test]
    fn parses_array_of_typedef_struct() {
        let unit = parse_unit("typedef struct Point { int x; } Point; Point arr[3];");
        let ExternalDeclaration::GlobalDeclaration(var) = &unit.top_level_items[1] else {
            panic!("expected variable declaration");
        };
        assert!(matches!(var.declarators[0].ty, Type::Array { .. }));
    }

    #[test]
    fn parses_struct_with_pointer_member() {
        let unit = parse_unit("struct Node { int value; struct Node *next; } n;");
        let ExternalDeclaration::GlobalDeclaration(var) = &unit.top_level_items[0] else {
            panic!("expected global declaration");
        };
        let Type::Struct { fields, .. } = &var.declarators[0].ty else {
            panic!("expected struct type");
        };
        assert_eq!(fields.len(), 2);
        assert!(matches!(fields[1].ty, Type::Pointer(_)));
    }
}
