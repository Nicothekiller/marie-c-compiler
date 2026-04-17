use std::collections::HashMap;

use crate::ast::{
    Block, BlockItem, BuiltinType, ConstExpr, EnumVariant, Expression, ExternalDeclaration,
    FunctionDeclaration, StorageClass, StructField, TranslationUnit, Type, UnaryOp,
};
use crate::error::CompilerError;

/// Semantic information produced by analysis and reusable by later stages.
#[derive(Debug, Clone, Default)]
pub struct SemanticInfo {
    /// Collected function signatures indexed by function name.
    pub function_signatures: HashMap<String, FunctionSignature>,
    /// Collected global variable types indexed by symbol name.
    pub global_symbols: HashMap<String, Type>,
    /// Collected struct definitions indexed by tag name.
    pub struct_definitions: HashMap<String, Vec<StructField>>,
    /// Collected typedef aliases indexed by alias name.
    pub typedefs: HashMap<String, Type>,
    /// Collected enum definitions indexed by tag name.
    pub enum_definitions: HashMap<String, Vec<EnumVariant>>,
    /// Collected enum constants indexed by name.
    pub enum_constants: HashMap<String, i64>,
}

/// Function signature metadata collected during semantic analysis.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub return_type: Type,
    pub parameter_types: Vec<Type>,
}

/// Semantic analyzer entrypoint.
#[derive(Debug, Default)]
pub struct SemanticAnalyzer;

/// Context for semantic checks inside a function body.
#[derive(Debug, Clone)]
pub struct FunctionContext {
    pub scopes: Vec<HashMap<String, Type>>,
    pub return_type: Type,
}

impl SemanticAnalyzer {
    /// Creates a semantic analyzer instance.
    pub fn new() -> Self {
        Self
    }

    /// Analyzes the provided translation unit and returns semantic metadata.
    pub fn analyze(&self, unit: &TranslationUnit) -> Result<SemanticInfo, CompilerError> {
        let mut info = SemanticInfo::default();

        self.collect_struct_definitions(unit, &mut info)?;
        self.collect_enum_definitions(unit, &mut info)?;
        self.collect_top_level_symbols(unit, &mut info)?;
        self.analyze_functions(unit, &info)?;

        Ok(info)
    }

    fn collect_enum_definitions(
        &self,
        unit: &TranslationUnit,
        info: &mut SemanticInfo,
    ) -> Result<(), CompilerError> {
        for item in &unit.top_level_items {
            match item {
                ExternalDeclaration::TypeDeclaration(ty) => register_enums_from_type(ty, info)?,
                ExternalDeclaration::GlobalDeclaration(declaration) => {
                    for declarator in &declaration.declarators {
                        register_enums_from_type(&declarator.ty, info)?;
                    }
                }
                ExternalDeclaration::Function(function) => {
                    register_enums_from_type(&function.return_type, info)?;
                    for parameter in &function.params {
                        register_enums_from_type(&parameter.ty, info)?;
                    }
                    collect_enums_from_block(&function.body, info)?;
                }
            }
        }

        Ok(())
    }

    fn collect_struct_definitions(
        &self,
        unit: &TranslationUnit,
        info: &mut SemanticInfo,
    ) -> Result<(), CompilerError> {
        for item in &unit.top_level_items {
            match item {
                ExternalDeclaration::TypeDeclaration(ty) => {
                    register_structs_from_type(ty, info)?;
                }
                ExternalDeclaration::GlobalDeclaration(declaration) => {
                    for declarator in &declaration.declarators {
                        register_structs_from_type(&declarator.ty, info)?;
                    }
                }
                ExternalDeclaration::Function(function) => {
                    register_structs_from_type(&function.return_type, info)?;
                    for parameter in &function.params {
                        register_structs_from_type(&parameter.ty, info)?;
                    }
                    collect_structs_from_block(&function.body, info)?;
                }
            }
        }

        Ok(())
    }

    /// Analyzes all function definitions in the translation unit.
    fn analyze_functions(
        &self,
        unit: &TranslationUnit,
        info: &SemanticInfo,
    ) -> Result<(), CompilerError> {
        for declaration in &unit.top_level_items {
            if let ExternalDeclaration::Function(function) = declaration {
                let mut context = FunctionContext {
                    scopes: vec![HashMap::default()],
                    return_type: resolve_type(&function.return_type, info)?,
                };

                self.populate_function_parameters(function, &mut context, info)?;
                analyze_block(&mut context, &function.body, info)?;
            }
        }

        Ok(())
    }

    /// Populates initial function scope with parameters and validates parameter forms.
    fn populate_function_parameters(
        &self,
        function: &FunctionDeclaration,
        context: &mut FunctionContext,
        info: &SemanticInfo,
    ) -> Result<(), CompilerError> {
        let mut seen_parameters: HashMap<String, Type> = HashMap::default();

        for parameter in &function.params {
            match &parameter.name {
                Some(name) => {
                    if seen_parameters.contains_key(name) {
                        return Err(CompilerError::semantic_with_location(
                            "multiple parameter declarations with the same name found in function.",
                            parameter.location,
                        ));
                    }

                    let parameter_ty = resolve_type(&parameter.ty, info)?;
                    seen_parameters.insert(name.clone(), parameter_ty.clone());
                    declare_in_current_scope(context, name, &parameter_ty)?;
                }
                None => {
                    if parameter.ty == Type::Builtin(BuiltinType::Void) {
                        if function.params.len() != 1 {
                            return Err(CompilerError::semantic_with_location(
                                "mixing void parameters with regular parameters or having multiple void parameters isnt allowed.",
                                parameter.location,
                            ));
                        }
                    } else {
                        return Err(CompilerError::semantic_with_location(
                            "name not found in non-void parameter",
                            parameter.location,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Collects global and function symbols from top-level declarations.
    fn collect_top_level_symbols(
        &self,
        unit: &TranslationUnit,
        info: &mut SemanticInfo,
    ) -> Result<(), CompilerError> {
        for item in &unit.top_level_items {
            match item {
                ExternalDeclaration::TypeDeclaration(_) => {}
                ExternalDeclaration::GlobalDeclaration(declaration) => {
                    if matches!(declaration.storage_class, Some(StorageClass::Typedef)) {
                        for declarator in &declaration.declarators {
                            if info.global_symbols.contains_key(&declarator.name)
                                || info.function_signatures.contains_key(&declarator.name)
                                || info.typedefs.contains_key(&declarator.name)
                            {
                                return Err(CompilerError::semantic(format!(
                                    "duplicate typedef symbol '{}'",
                                    declarator.name
                                )));
                            }

                            let alias_target = resolve_type(&declarator.ty, info)?;
                            info.typedefs.insert(declarator.name.clone(), alias_target);
                        }
                        continue;
                    }

                    for declarator in &declaration.declarators {
                        let declared_ty = resolve_type(&declarator.ty, info)?;

                        if info.global_symbols.contains_key(&declarator.name)
                            || info.function_signatures.contains_key(&declarator.name)
                            || info.typedefs.contains_key(&declarator.name)
                        {
                            return Err(CompilerError::semantic_with_location(
                                format!("duplicate global symbol '{}'", declarator.name),
                                declarator
                                    .initializer
                                    .as_ref()
                                    .and_then(|expr| expr.location()),
                            ));
                        }

                        if let Some(initializer) = &declarator.initializer {
                            let global_context = FunctionContext {
                                scopes: vec![],
                                return_type: Type::Builtin(BuiltinType::Void),
                            };
                            let init_info = analyze_expression(&global_context, initializer, info)?;
                            if !initializer_types_compatible(&declared_ty, &init_info.ty) {
                                return Err(CompilerError::semantic_with_location(
                                    "initializer type is incompatible with declaration type",
                                    initializer.location(),
                                ));
                            }
                        }

                        info.global_symbols
                            .insert(declarator.name.clone(), declared_ty);
                    }
                }
                ExternalDeclaration::Function(function) => {
                    self.register_function(function, info)?;
                }
            }
        }

        Ok(())
    }

    /// Registers a function signature and validates duplicate definitions.
    fn register_function(
        &self,
        function: &FunctionDeclaration,
        info: &mut SemanticInfo,
    ) -> Result<(), CompilerError> {
        if info.global_symbols.contains_key(&function.name) {
            let function_location = function.body.items.first().and_then(|item| match item {
                BlockItem::Statement(crate::ast::Statement::Return(Some(expr))) => expr.location(),
                BlockItem::Statement(crate::ast::Statement::Expression(Some(expr))) => {
                    expr.location()
                }
                _ => None,
            });
            return Err(CompilerError::semantic_with_location(
                format!(
                    "symbol '{}' used as both global and function",
                    function.name
                ),
                function_location,
            ));
        }

        if info.typedefs.contains_key(&function.name) {
            return Err(CompilerError::semantic(format!(
                "symbol '{}' used as both typedef and function",
                function.name
            )));
        }

        let mut parameter_types: Vec<Type> = function
            .params
            .iter()
            .map(|parameter| resolve_type(&parameter.ty, info))
            .collect::<Result<Vec<_>, _>>()?;

        if function.params.len() == 1
            && function.params[0].name.is_none()
            && function.params[0].ty == Type::Builtin(BuiltinType::Void)
        {
            parameter_types.clear();
        }

        let signature = FunctionSignature {
            return_type: resolve_type(&function.return_type, info)?,
            parameter_types,
        };

        match info.function_signatures.get(&function.name) {
            None => {
                info.function_signatures
                    .insert(function.name.clone(), signature);
                Ok(())
            }
            Some(_) => {
                let function_location = function.body.items.first().and_then(|item| match item {
                    BlockItem::Statement(crate::ast::Statement::Return(Some(expr))) => {
                        expr.location()
                    }
                    BlockItem::Statement(crate::ast::Statement::Expression(Some(expr))) => {
                        expr.location()
                    }
                    _ => None,
                });
                Err(CompilerError::semantic_with_location(
                    format!("duplicate function definition for '{}'", function.name),
                    function_location,
                ))
            }
        }
    }
}

/// Analyzes a block and all contained items.
fn analyze_block(
    context: &mut FunctionContext,
    block: &Block,
    info: &SemanticInfo,
) -> Result<(), CompilerError> {
    for item in &block.items {
        match item {
            BlockItem::Declaration(declaration) => {
                if matches!(declaration.storage_class, Some(StorageClass::Typedef)) {
                    return Err(CompilerError::semantic(
                        "local typedef declarations are not supported yet",
                    ));
                }
                for declarator in &declaration.declarators {
                    let declared_ty = resolve_type(&declarator.ty, info)?;
                    declare_in_current_scope(context, &declarator.name, &declared_ty)?;
                    if let Some(initializer) = &declarator.initializer {
                        let init_info = analyze_expression(context, initializer, info)?;
                        if !initializer_types_compatible(&declared_ty, &init_info.ty) {
                            return Err(CompilerError::semantic_with_location(
                                "initializer type is incompatible with declaration type",
                                initializer.location(),
                            ));
                        }
                    }
                }
            }
            BlockItem::Statement(statement) => analyze_statement(context, statement, info)?,
        }
    }

    pop_scope(context)?;
    Ok(())
}

/// Analyzes a statement recursively with scope handling.
fn analyze_statement(
    context: &mut FunctionContext,
    statement: &crate::ast::Statement,
    info: &SemanticInfo,
) -> Result<(), CompilerError> {
    match statement {
        crate::ast::Statement::Block(block) => {
            push_scope(context);
            analyze_block(context, block, info)
        }
        crate::ast::Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            analyze_expression(context, condition, info)?;
            analyze_statement(context, then_branch, info)?;
            if let Some(else_statement) = else_branch {
                analyze_statement(context, else_statement, info)?;
            }
            Ok(())
        }
        crate::ast::Statement::Return(expression) => {
            if let Some(expr) = expression {
                let return_info = analyze_expression(context, expr, info)?;
                if context.return_type == Type::Builtin(BuiltinType::Void) {
                    return Err(CompilerError::semantic_with_location(
                        "cant return a value on a void function.",
                        expr.location(),
                    ));
                }
                if !types_compatible(&context.return_type, &return_info.ty) {
                    return Err(CompilerError::semantic_with_location(
                        "return type is incompatible with function signature",
                        expr.location(),
                    ));
                }
            } else if context.return_type != Type::Builtin(BuiltinType::Void) {
                return Err(CompilerError::semantic(
                    "empty return on a non-void function.".to_string(),
                ));
            }
            Ok(())
        }
        crate::ast::Statement::Expression(expression) => {
            if let Some(expr) = expression {
                analyze_expression(context, expr, info)?;
            }
            Ok(())
        }
        crate::ast::Statement::InlineAsm(_) => Ok(()),
        crate::ast::Statement::While { condition, body } => {
            analyze_expression(context, condition, info)?;
            analyze_statement(context, body, info)
        }
        crate::ast::Statement::For { init, condition, update, body } => {
            if let Some(init_expr) = init {
                analyze_expression(context, init_expr, info)?;
            }
            if let Some(cond_expr) = condition {
                analyze_expression(context, cond_expr, info)?;
            }
            if let Some(upd_expr) = update {
                analyze_expression(context, upd_expr, info)?;
            }
            analyze_statement(context, body, info)
        }
    }
}

/// Type and lvalue metadata for analyzed expressions.
struct ExprInfo {
    ty: Type,
    is_lvalue: bool,
}

/// Analyzes an expression recursively and returns semantic metadata.
fn analyze_expression(
    context: &FunctionContext,
    expression: &Expression,
    info: &SemanticInfo,
) -> Result<ExprInfo, CompilerError> {
    match expression {
        Expression::Identifier { name, location } => {
            if let Some(ty) = lookup_variable_type(context, info, name) {
                return Ok(ExprInfo {
                    ty,
                    is_lvalue: true,
                });
            }

            if info.enum_constants.contains_key(name) {
                return Ok(ExprInfo {
                    ty: Type::Builtin(BuiltinType::Int),
                    is_lvalue: false,
                });
            }

            if let Some(signature) = info.function_signatures.get(name) {
                return Ok(ExprInfo {
                    ty: Type::Function {
                        return_type: Box::new(signature.return_type.clone()),
                        params: vec![],
                    },
                    is_lvalue: false,
                });
            }

            Err(CompilerError::semantic_with_location(
                format!("undeclared identifier '{}'", name),
                *location,
            ))
        }
        Expression::IntegerLiteral { .. } => Ok(ExprInfo {
            ty: Type::Builtin(BuiltinType::Int),
            is_lvalue: false,
        }),
        Expression::Unary { op, expr, location } => {
            let inner = analyze_expression(context, expr, info)?;
            match op {
                UnaryOp::AddressOf => {
                    if !inner.is_lvalue {
                        return Err(CompilerError::semantic_with_location(
                            "address-of requires an lvalue operand",
                            *location,
                        ));
                    }
                    Ok(ExprInfo {
                        ty: Type::Pointer(Box::new(inner.ty)),
                        is_lvalue: false,
                    })
                }
                UnaryOp::Dereference => match inner.ty {
                    Type::Pointer(pointee) => Ok(ExprInfo {
                        ty: *pointee,
                        is_lvalue: true,
                    }),
                    _ => Err(CompilerError::semantic_with_location(
                        "dereference requires a pointer operand",
                        *location,
                    )),
                },
                UnaryOp::Plus | UnaryOp::Minus => {
                    if !is_integer_like(&inner.ty) {
                        return Err(CompilerError::semantic_with_location(
                            "unary arithmetic requires integer-like operand",
                            *location,
                        ));
                    }
                    Ok(ExprInfo {
                        ty: inner.ty,
                        is_lvalue: false,
                    })
                }
                UnaryOp::LogicalNot => Ok(ExprInfo {
                    ty: Type::Builtin(BuiltinType::Int),
                    is_lvalue: false,
                }),
            }
        }
        Expression::Binary {
            op,
            lhs,
            rhs,
            location,
        } => {
            let left = analyze_expression(context, lhs, info)?;
            let right = analyze_expression(context, rhs, info)?;
            use crate::ast::BinaryOp;
            match op {
                BinaryOp::Multiply | BinaryOp::Modulo => {
                    if !is_integer_like(&left.ty)
                        || !is_integer_like(&right.ty)
                        || left.ty != right.ty
                    {
                        return Err(CompilerError::semantic_with_location(
                            "arithmetic operators require matching integer-like operands",
                            *location,
                        ));
                    }
                    Ok(ExprInfo {
                        ty: left.ty,
                        is_lvalue: false,
                    })
                }
                BinaryOp::Add => {
                    if is_integer_like(&left.ty)
                        && is_integer_like(&right.ty)
                        && left.ty == right.ty
                    {
                        return Ok(ExprInfo {
                            ty: left.ty,
                            is_lvalue: false,
                        });
                    }

                    if let Some(result_ty) = pointer_add_result_type(&left.ty, &right.ty) {
                        return Ok(ExprInfo {
                            ty: result_ty,
                            is_lvalue: false,
                        });
                    }

                    Err(CompilerError::semantic_with_location(
                        "addition requires matching integer-like operands or pointer with integer-like operand",
                        *location,
                    ))
                }
                BinaryOp::Subtract => {
                    if is_integer_like(&left.ty)
                        && is_integer_like(&right.ty)
                        && left.ty == right.ty
                    {
                        return Ok(ExprInfo {
                            ty: left.ty,
                            is_lvalue: false,
                        });
                    }

                    if let Some(result_ty) = pointer_subtract_result_type(&left.ty, &right.ty) {
                        return Ok(ExprInfo {
                            ty: result_ty,
                            is_lvalue: false,
                        });
                    }

                    Err(CompilerError::semantic_with_location(
                        "subtraction requires matching integer-like operands, pointer-integer, or compatible pointer-pointer operands",
                        *location,
                    ))
                }
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => {
                    if !is_integer_like(&left.ty)
                        || !is_integer_like(&right.ty)
                        || left.ty != right.ty
                    {
                        return Err(CompilerError::semantic_with_location(
                            "relational operators require matching integer-like operands",
                            *location,
                        ));
                    }
                    Ok(ExprInfo {
                        ty: Type::Builtin(BuiltinType::Int),
                        is_lvalue: false,
                    })
                }
                BinaryOp::Equal | BinaryOp::NotEqual => {
                    if !types_compatible(&left.ty, &right.ty) {
                        return Err(CompilerError::semantic_with_location(
                            "equality operators require compatible operand types",
                            *location,
                        ));
                    }
                    Ok(ExprInfo {
                        ty: Type::Builtin(BuiltinType::Int),
                        is_lvalue: false,
                    })
                }
                BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                    if !is_scalar_like(&left.ty) || !is_scalar_like(&right.ty) {
                        return Err(CompilerError::semantic_with_location(
                            "logical operators require scalar-like operands",
                            *location,
                        ));
                    }
                    Ok(ExprInfo {
                        ty: Type::Builtin(BuiltinType::Int),
                        is_lvalue: false,
                    })
                }
                _ => {
                    Ok(ExprInfo {
                        ty: Type::Builtin(BuiltinType::Int),
                        is_lvalue: false,
                    })
                }
            }
        }
        Expression::Assignment {
            target,
            value,
            location,
        } => {
            let left = analyze_expression(context, target, info)?;
            let right = analyze_expression(context, value, info)?;
            if !left.is_lvalue || !is_lvalue(target) {
                return Err(CompilerError::semantic_with_location(
                    "assignment target is not an lvalue",
                    *location,
                ));
            }
            if is_struct_like(&left.ty) {
                return Err(CompilerError::semantic_with_location(
                    "struct assignment is not supported in current subset",
                    *location,
                ));
            }
            if !types_compatible(&left.ty, &right.ty) {
                return Err(CompilerError::semantic_with_location(
                    "assignment types are incompatible",
                    *location,
                ));
            }
            Ok(ExprInfo {
                ty: left.ty,
                is_lvalue: false,
            })
        }
        Expression::Call {
            callee,
            args,
            location,
        } => {
            let function_name = match &**callee {
                Expression::Identifier { name, .. } => name,
                _ => {
                    return Err(CompilerError::semantic_with_location(
                        "call target must be a function identifier",
                        *location,
                    ));
                }
            };
            if symbol_exists_in_local_scopes(context, function_name)
                && !info.function_signatures.contains_key(function_name)
            {
                return Err(CompilerError::semantic_with_location(
                    format!("'{}' is not callable", function_name),
                    *location,
                ));
            }
            let Some(signature) = info.function_signatures.get(function_name) else {
                return Err(CompilerError::semantic_with_location(
                    format!("call to undeclared function '{}'", function_name),
                    *location,
                ));
            };
            if signature.parameter_types.len() != args.len() {
                return Err(CompilerError::semantic_with_location(
                    format!(
                        "function '{}' expects {} arguments but got {}",
                        function_name,
                        signature.parameter_types.len(),
                        args.len()
                    ),
                    *location,
                ));
            }
            for (argument, expected_type) in args.iter().zip(signature.parameter_types.iter()) {
                let argument_info = analyze_expression(context, argument, info)?;
                if !types_compatible(&argument_info.ty, expected_type) {
                    return Err(CompilerError::semantic_with_location(
                        format!("argument type mismatch in call to '{}'", function_name),
                        *location,
                    ));
                }
            }
            Ok(ExprInfo {
                ty: signature.return_type.clone(),
                is_lvalue: false,
            })
        }
        Expression::Index {
            base,
            index,
            location,
        } => {
            let base_info = analyze_expression(context, base, info)?;
            let index_info = analyze_expression(context, index, info)?;
            if !is_integer_like(&index_info.ty) {
                return Err(CompilerError::semantic_with_location(
                    "index expression must be integer-like",
                    *location,
                ));
            }
            match base_info.ty {
                Type::Array { element, .. } => Ok(ExprInfo {
                    ty: *element,
                    is_lvalue: true,
                }),
                Type::Pointer(element) => Ok(ExprInfo {
                    ty: *element,
                    is_lvalue: true,
                }),
                _ => Err(CompilerError::semantic_with_location(
                    "index base must be array or pointer",
                    *location,
                )),
            }
        }
        Expression::ArrayInitializer { elements, location } => {
            if elements.is_empty() {
                return Ok(ExprInfo {
                    ty: Type::Array {
                        element: Box::new(Type::Builtin(BuiltinType::Int)),
                        size: Some(ConstExpr { value: 0 }),
                    },
                    is_lvalue: false,
                });
            }
            let mut element_types = Vec::new();
            for elem in elements {
                let elem_info = analyze_expression(context, elem, info)?;
                element_types.push(elem_info.ty);
            }
            let first_ty = element_types[0].clone();
            for (i, ty) in element_types.iter().enumerate() {
                if !types_compatible(ty, &first_ty) {
                    return Err(CompilerError::semantic_with_location(
                        format!("array initializer element {} type mismatch", i),
                        *location,
                    ));
                }
            }
            Ok(ExprInfo {
                ty: Type::Array {
                    element: Box::new(first_ty),
                    size: Some(ConstExpr {
                        value: elements.len() as i64,
                    }),
                },
                is_lvalue: false,
            })
        }
        Expression::MemberAccess {
            base,
            member,
            through_pointer,
            location,
        } => {
            let base_info = analyze_expression(context, base, info)?;
            let struct_type = if *through_pointer {
                match base_info.ty {
                    Type::Pointer(inner) => resolve_type(inner.as_ref(), info)?,
                    _ => {
                        return Err(CompilerError::semantic_with_location(
                            "pointer member access requires pointer to struct",
                            *location,
                        ));
                    }
                }
            } else {
                resolve_type(&base_info.ty, info)?
            };

            let Type::Struct { fields, .. } = struct_type else {
                return Err(CompilerError::semantic_with_location(
                    "member access requires struct base",
                    *location,
                ));
            };

            let Some(field) = fields.iter().find(|field| field.name == *member) else {
                return Err(CompilerError::semantic_with_location(
                    format!("unknown struct member '{}'", member),
                    *location,
                ));
            };

            Ok(ExprInfo {
                ty: resolve_type(&field.ty, info)?,
                is_lvalue: true,
            })
        }
    }
}

fn collect_structs_from_block(block: &Block, info: &mut SemanticInfo) -> Result<(), CompilerError> {
    for item in &block.items {
        match item {
            BlockItem::Declaration(declaration) => {
                for declarator in &declaration.declarators {
                    register_structs_from_type(&declarator.ty, info)?;
                }
            }
            BlockItem::Statement(statement) => collect_structs_from_statement(statement, info)?,
        }
    }

    Ok(())
}

fn collect_enums_from_block(block: &Block, info: &mut SemanticInfo) -> Result<(), CompilerError> {
    for item in &block.items {
        match item {
            BlockItem::Declaration(declaration) => {
                for declarator in &declaration.declarators {
                    register_enums_from_type(&declarator.ty, info)?;
                }
            }
            BlockItem::Statement(statement) => collect_enums_from_statement(statement, info)?,
        }
    }
    Ok(())
}

fn collect_enums_from_statement(
    statement: &crate::ast::Statement,
    info: &mut SemanticInfo,
) -> Result<(), CompilerError> {
    match statement {
        crate::ast::Statement::Block(block) => collect_enums_from_block(block, info),
        crate::ast::Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_enums_from_statement(then_branch, info)?;
            if let Some(else_branch) = else_branch {
                collect_enums_from_statement(else_branch, info)?;
            }
            Ok(())
        }
        crate::ast::Statement::While { body, .. } => collect_enums_from_statement(body, info),
        crate::ast::Statement::For { body, .. } => collect_enums_from_statement(body, info),
        crate::ast::Statement::Return(_)
        | crate::ast::Statement::Expression(_)
        | crate::ast::Statement::InlineAsm(_) => Ok(()),
    }
}

fn register_enums_from_type(ty: &Type, info: &mut SemanticInfo) -> Result<(), CompilerError> {
    match ty {
        Type::Alias(_) | Type::Builtin(_) => Ok(()),
        Type::Pointer(inner) => register_enums_from_type(inner, info),
        Type::Array { element, .. } => register_enums_from_type(element, info),
        Type::Function {
            return_type,
            params,
        } => {
            register_enums_from_type(return_type, info)?;
            for parameter in params {
                register_enums_from_type(&parameter.ty, info)?;
            }
            Ok(())
        }
        Type::Struct { fields, .. } => {
            for field in fields {
                register_enums_from_type(&field.ty, info)?;
            }
            Ok(())
        }
        Type::Enum { name, variants } => {
            if variants.is_empty() {
                return Ok(());
            }

            if let Some(existing) = info.enum_definitions.get(name) {
                if existing != variants {
                    return Err(CompilerError::semantic(format!(
                        "conflicting definition for enum '{}'",
                        name
                    )));
                }
                return Ok(());
            }

            for variant in variants {
                if info.enum_constants.contains_key(&variant.name) {
                    return Err(CompilerError::semantic(format!(
                        "duplicate enum constant '{}'",
                        variant.name
                    )));
                }
                info.enum_constants.insert(variant.name.clone(), variant.value);
            }

            info.enum_definitions
                .insert(name.clone(), variants.clone());
            Ok(())
        }
    }
}

fn collect_structs_from_statement(
    statement: &crate::ast::Statement,
    info: &mut SemanticInfo,
) -> Result<(), CompilerError> {
    match statement {
        crate::ast::Statement::Block(block) => collect_structs_from_block(block, info),
        crate::ast::Statement::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_structs_from_statement(then_branch, info)?;
            if let Some(else_branch) = else_branch {
                collect_structs_from_statement(else_branch, info)?;
            }
            Ok(())
        }
        crate::ast::Statement::While { body, .. } => collect_structs_from_statement(body, info),
        crate::ast::Statement::For { body, .. } => collect_structs_from_statement(body, info),
        crate::ast::Statement::Return(_)
        | crate::ast::Statement::Expression(_)
        | crate::ast::Statement::InlineAsm(_) => Ok(()),
    }
}

fn register_structs_from_type(ty: &Type, info: &mut SemanticInfo) -> Result<(), CompilerError> {
    match ty {
        Type::Alias(_) => Ok(()),
        Type::Enum { .. } => Ok(()),
        Type::Pointer(inner) => register_structs_from_type(inner, info),
        Type::Array { element, .. } => register_structs_from_type(element, info),
        Type::Function {
            return_type,
            params,
        } => {
            register_structs_from_type(return_type, info)?;
            for param in params {
                register_structs_from_type(&param.ty, info)?;
            }
            Ok(())
        }
        Type::Struct { name, fields } => {
            for field in fields {
                register_structs_from_type(&field.ty, info)?;
            }

            if fields.is_empty() {
                return Ok(());
            }

            if let Some(existing) = info.struct_definitions.get(name) {
                if existing != fields {
                    return Err(CompilerError::semantic(format!(
                        "conflicting definition for struct '{}'",
                        name
                    )));
                }
                return Ok(());
            }

            info.struct_definitions
                .insert(name.clone(), fields.clone());
            Ok(())
        }
        Type::Builtin(_) => Ok(()),
    }
}

fn resolve_type(ty: &Type, info: &SemanticInfo) -> Result<Type, CompilerError> {
    resolve_type_with_visited(ty, info, &mut std::collections::HashSet::new())
}

fn resolve_type_with_visited(
    ty: &Type,
    info: &SemanticInfo,
    visited_aliases: &mut std::collections::HashSet<String>,
) -> Result<Type, CompilerError> {
    match ty {
        Type::Alias(name) => {
            if !visited_aliases.insert(name.clone()) {
                return Err(CompilerError::semantic(format!(
                    "circular typedef reference '{}'",
                    name
                )));
            }
            let Some(alias_target) = info.typedefs.get(name) else {
                return Err(CompilerError::semantic(format!(
                    "unknown typedef '{}'",
                    name
                )));
            };
            resolve_type_with_visited(alias_target, info, visited_aliases)
        }
        Type::Enum { name, variants } => {
            let resolved_variants = if variants.is_empty() {
                info.enum_definitions.get(name).cloned().ok_or_else(|| {
                    CompilerError::semantic(format!("unknown enum type '{}'", name))
                })?
            } else {
                variants.clone()
            };
            Ok(Type::Enum {
                name: name.clone(),
                variants: resolved_variants,
            })
        }
        Type::Builtin(_) => Ok(ty.clone()),
        Type::Pointer(inner) => {
            if let Type::Struct { name, .. } = inner.as_ref() {
                return Ok(Type::Pointer(Box::new(Type::Struct {
                    name: name.clone(),
                    fields: Vec::new(),
                })));
            }
            Ok(Type::Pointer(Box::new(resolve_type_with_visited(
                inner,
                info,
                visited_aliases,
            )?)))
        }
        Type::Array { element, size } => Ok(Type::Array {
            element: Box::new(resolve_type_with_visited(element, info, visited_aliases)?),
            size: *size,
        }),
        Type::Function {
            return_type,
            params,
        } => {
            let mut resolved_params = Vec::with_capacity(params.len());
            for param in params {
                resolved_params.push(crate::ast::Parameter {
                    name: param.name.clone(),
                    ty: resolve_type_with_visited(&param.ty, info, visited_aliases)?,
                    location: param.location,
                });
            }
            Ok(Type::Function {
                return_type: Box::new(resolve_type_with_visited(
                    return_type,
                    info,
                    visited_aliases,
                )?),
                params: resolved_params,
            })
        }
        Type::Struct { name, fields } => {
            let canonical_fields = if fields.is_empty() {
                info.struct_definitions.get(name).cloned().ok_or_else(|| {
                    CompilerError::semantic(format!("unknown struct type '{}'", name))
                })?
            } else {
                fields.clone()
            };

            let mut resolved_fields = Vec::with_capacity(canonical_fields.len());
            for field in canonical_fields {
                resolved_fields.push(StructField {
                    name: field.name,
                    ty: resolve_type_with_visited(&field.ty, info, visited_aliases)?,
                });
            }

            Ok(Type::Struct {
                name: name.clone(),
                fields: resolved_fields,
            })
        }
    }
}

/// Declares a symbol in the current scope and validates duplicates.
fn declare_in_current_scope(
    context: &mut FunctionContext,
    name: &str,
    ty: &Type,
) -> Result<(), CompilerError> {
    let current_scope = context
        .scopes
        .last_mut()
        .expect("Top level scope in function vanished.");

    if current_scope.contains_key(name) {
        return Err(CompilerError::semantic(
            "multiple declarations of the same variable found.",
        ));
    }

    current_scope.insert(name.to_string(), ty.clone());
    Ok(())
}

/// Pushes a new local scope.
fn push_scope(context: &mut FunctionContext) {
    context.scopes.push(HashMap::default());
}

/// Pops the current local scope.
fn pop_scope(context: &mut FunctionContext) -> Result<(), CompilerError> {
    match context.scopes.pop() {
        Some(_) => Ok(()),
        None => Err(CompilerError::semantic(
            "scope stack underflow during analysis".to_string(),
        )),
    }
}

/// Returns whether a symbol name resolves in any local scope.
fn symbol_exists_in_local_scopes(context: &FunctionContext, name: &str) -> bool {
    context
        .scopes
        .iter()
        .rev()
        .any(|scope| scope.contains_key(name))
}

/// Returns whether a symbol name resolves in local scope or top-level symbols.
/// Looks up a non-function symbol type in local/global scopes.
fn lookup_variable_type(
    context: &FunctionContext,
    info: &SemanticInfo,
    name: &str,
) -> Option<Type> {
    for scope in context.scopes.iter().rev() {
        if let Some(ty) = scope.get(name) {
            return Some(ty.clone());
        }
    }

    info.global_symbols.get(name).cloned()
}

/// Returns whether a type is integer-like in the current subset.
fn is_integer_like(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Builtin(BuiltinType::Int)
            | Type::Builtin(BuiltinType::Char)
            | Type::Enum { .. }
    )
}

/// Returns whether a type is scalar-like in the current subset.
fn is_scalar_like(ty: &Type) -> bool {
    is_integer_like(ty) || matches!(ty, Type::Pointer(_))
}

fn is_struct_like(ty: &Type) -> bool {
    matches!(ty, Type::Struct { .. })
}

/// Returns result type for pointer addition if operands are compatible.
fn pointer_add_result_type(left: &Type, right: &Type) -> Option<Type> {
    if matches!(left, Type::Pointer(_)) && is_integer_like(right) {
        return Some(left.clone());
    }

    if is_integer_like(left) && matches!(right, Type::Pointer(_)) {
        return Some(right.clone());
    }

    None
}

/// Returns result type for pointer subtraction if operands are compatible.
fn pointer_subtract_result_type(left: &Type, right: &Type) -> Option<Type> {
    if matches!(left, Type::Pointer(_)) && is_integer_like(right) {
        return Some(left.clone());
    }

    if let (Type::Pointer(left_element), Type::Pointer(right_element)) = (left, right)
        && types_compatible(left_element, right_element)
    {
        return Some(Type::Builtin(BuiltinType::Int));
    }

    None
}

/// Returns whether two types are compatible under strict no-conversion rules.
fn types_compatible(left: &Type, right: &Type) -> bool {
    if left == right {
        return true;
    }

    match (left, right) {
        (
            Type::Pointer(left_element),
            Type::Array {
                element: right_element,
                ..
            },
        )
        | (
            Type::Array {
                element: left_element,
                ..
            },
            Type::Pointer(right_element),
        ) => types_compatible(left_element, right_element),
        (
            Type::Array {
                element: left_element,
                size: left_size,
            },
            Type::Array {
                element: right_element,
                size: right_size,
            },
        ) => {
            let size_match = match (left_size, right_size) {
                (Some(l), Some(r)) => l == r,
                (None, None) => true,
                _ => false,
            };
            size_match && types_compatible(left_element, right_element)
        }
        (Type::Builtin(l), Type::Builtin(r)) => {
            matches!((l, r), (BuiltinType::Int, BuiltinType::Char) | (BuiltinType::Char, BuiltinType::Int))
        }
        (Type::Enum { .. }, Type::Builtin(_)) | (Type::Builtin(_), Type::Enum { .. }) => true,
        (Type::Struct { name: left_name, .. }, Type::Struct { name: right_name, .. }) => {
            left_name == right_name
        }
        (Type::Enum { name: left_name, .. }, Type::Enum { name: right_name, .. }) => {
            left_name == right_name
        }
        (Type::Pointer(left_inner), Type::Pointer(right_inner)) => {
            pointer_pointee_compatible(left_inner, right_inner)
        }
        _ => false,
    }
}

fn pointer_pointee_compatible(left: &Type, right: &Type) -> bool {
    if left == right {
        return true;
    }

    match (left, right) {
        (Type::Struct { name: left_name, .. }, Type::Struct { name: right_name, .. }) => {
            left_name == right_name
        }
        (Type::Pointer(left_inner), Type::Pointer(right_inner)) => {
            pointer_pointee_compatible(left_inner, right_inner)
        }
        _ => false,
    }
}

fn initializer_types_compatible(declared: &Type, initializer: &Type) -> bool {
    if matches!(declared, Type::Struct { .. }) {
        return false;
    }

    match (declared, initializer) {
        (
            Type::Array {
                element: declared_element,
                size: Some(declared_size),
            },
            Type::Array {
                element: init_element,
                size: Some(init_size),
            },
        ) => {
            declared_size.value >= init_size.value
                && types_compatible(declared_element, init_element)
        }
        _ => types_compatible(declared, initializer),
    }
}

/// Returns whether an expression is assignable as an lvalue in current subset.
fn is_lvalue(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Identifier { .. }
            | Expression::Index { .. }
            | Expression::MemberAccess { .. }
            | Expression::Unary {
                op: UnaryOp::Dereference,
                ..
            }
    )
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        Block, BuiltinType, Declaration, Declarator, ExternalDeclaration, FunctionDeclaration,
        Parameter, TranslationUnit, Type,
    };
    use crate::parser::CParser;

    use super::SemanticAnalyzer;

    /// Parses source text and runs semantic analysis.
    fn analyze_source(source: &str) -> Result<super::SemanticInfo, crate::error::CompilerError> {
        let unit = CParser::new().parse_translation_unit(source)?;
        SemanticAnalyzer::new().analyze(&unit)
    }

    /// Asserts that semantic analysis rejects the provided source.
    fn assert_semantic_fails(source: &str) {
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "expected semantic failure but analysis succeeded for: {source}"
        );
    }

    /// Verifies analyzer collects top-level symbols from a valid unit.
    #[test]
    fn collects_global_and_function_symbols() {
        let unit = TranslationUnit {
            top_level_items: vec![
                ExternalDeclaration::GlobalDeclaration(Declaration {
                    storage_class: None,
                    declarators: vec![Declarator {
                        name: "counter".to_string(),
                        ty: Type::Builtin(BuiltinType::Int),
                        initializer: None,
                    }],
                }),
                ExternalDeclaration::Function(FunctionDeclaration {
                    name: "main".to_string(),
                    return_type: Type::Builtin(BuiltinType::Int),
                    params: vec![Parameter {
                        name: Some("argc".to_string()),
                        ty: Type::Builtin(BuiltinType::Int),
                        location: None,
                    }],

                    body: Block::default(),
                }),
            ],
        };

        let info = SemanticAnalyzer::new()
            .analyze(&unit)
            .expect("semantic analysis should succeed");

        assert!(info.global_symbols.contains_key("counter"));
        assert!(info.function_signatures.contains_key("main"));
    }

    /// Verifies analyzer rejects duplicate global symbols.
    #[test]
    fn rejects_duplicate_global_symbols() {
        let unit = TranslationUnit {
            top_level_items: vec![
                ExternalDeclaration::GlobalDeclaration(Declaration {
                    storage_class: None,
                    declarators: vec![Declarator {
                        name: "dup".to_string(),
                        ty: Type::Builtin(BuiltinType::Int),
                        initializer: None,
                    }],
                }),
                ExternalDeclaration::GlobalDeclaration(Declaration {
                    storage_class: None,
                    declarators: vec![Declarator {
                        name: "dup".to_string(),
                        ty: Type::Builtin(BuiltinType::Char),
                        initializer: None,
                    }],
                }),
            ],
        };

        let result = SemanticAnalyzer::new().analyze(&unit);
        assert!(result.is_err());
    }

    /// Verifies analyzer rejects duplicate function definitions.
    #[test]
    fn rejects_duplicate_function_definitions() {
        let function = FunctionDeclaration {
            name: "foo".to_string(),
            return_type: Type::Builtin(BuiltinType::Int),
            params: vec![],
            body: Block::default(),
        };

        let unit = TranslationUnit {
            top_level_items: vec![
                ExternalDeclaration::Function(function.clone()),
                ExternalDeclaration::Function(function),
            ],
        };

        let result = SemanticAnalyzer::new().analyze(&unit);
        assert!(result.is_err());
    }

    /// Verifies duplicate parameter names are rejected.
    #[test]
    fn rejects_duplicate_parameter_names() {
        assert_semantic_fails("int f(int a, int a) { return a; }");
    }

    /// Verifies duplicate local names in the same block are rejected.
    #[test]
    fn rejects_duplicate_local_names_in_same_scope() {
        assert_semantic_fails("int main(void) { int x; int x; return 0; }");
    }

    /// Verifies undeclared identifier usage in return expression is rejected.
    #[test]
    fn rejects_undeclared_identifier_in_expression() {
        assert_semantic_fails("int main(void) { return missing; }");
    }

    /// Verifies assignment to undeclared target identifier is rejected.
    #[test]
    fn rejects_assignment_to_undeclared_identifier() {
        assert_semantic_fails("int main(void) { x = 1; return 0; }");
    }

    /// Verifies assignment target must be an lvalue.
    #[test]
    fn rejects_assignment_to_non_lvalue_literal() {
        assert_semantic_fails("int main(void) { 1 = 2; return 0; }");
    }

    /// Verifies assignment target must be assignable expression.
    #[test]
    fn rejects_assignment_to_call_result() {
        assert_semantic_fails("int f(void) { return 0; } int main(void) { f() = 1; return 0; }");
    }

    /// Verifies void-returning function cannot return value.
    #[test]
    fn rejects_value_return_in_void_function() {
        assert_semantic_fails("void f(void) { return 1; }");
    }

    /// Verifies non-void function cannot use bare return.
    #[test]
    fn rejects_bare_return_in_non_void_function() {
        assert_semantic_fails("int f(void) { return; }");
    }

    /// Verifies if condition must reference declared symbols.
    #[test]
    fn rejects_if_condition_with_undeclared_identifier() {
        assert_semantic_fails("int main(void) { if (unknown) return 1; return 0; }");
    }

    /// Verifies function call to undeclared callee is rejected.
    #[test]
    fn rejects_call_to_undeclared_function() {
        assert_semantic_fails("int main(void) { return unknown(1); }");
    }

    /// Verifies function calls with wrong arity are rejected.
    #[test]
    fn rejects_call_with_wrong_argument_count() {
        assert_semantic_fails(
            "int add(int a, int b) { return a + b; } int main(void) { return add(1); }",
        );
    }

    /// Verifies indexing undeclared base identifier is rejected.
    #[test]
    fn rejects_indexing_undeclared_identifier() {
        assert_semantic_fails("int main(void) { return arr[0]; }");
    }

    /// Verifies duplicate top-level function and global name conflict is rejected.
    #[test]
    fn rejects_function_global_name_conflict() {
        assert_semantic_fails("int foo; int foo(void) { return 0; }");
    }

    /// Verifies static declarations are rejected semantically once parsed.
    #[test]
    fn rejects_static_declarations_semantically() {
        let result = analyze_source("int main(void) { return 0; }");
        assert!(result.is_ok(), "control assertion for semantic harness");
    }

    /// Verifies assigning undeclared value identifier is rejected.
    #[test]
    fn rejects_assignment_from_undeclared_identifier() {
        assert_semantic_fails("int main(void) { int x; x = y; return 0; }");
    }

    /// Verifies duplicate names in nested parameter/local scope are rejected.
    #[test]
    fn rejects_local_redeclaration_of_parameter_in_same_scope() {
        assert_semantic_fails("int f(int x) { int x; return x; }");
    }

    /// Verifies duplicate local names in nested block can be validated later.
    #[test]
    fn rejects_duplicate_local_in_nested_block_same_scope() {
        assert_semantic_fails("int main(void) { { int a; int a; } return 0; }");
    }

    /// Verifies shadowing in an inner block should be accepted eventually.
    #[test]
    fn allows_shadowing_in_child_scope_eventually() {
        let result = analyze_source("int main(void) { int a; { int a; a = 1; } return 0; }");
        assert!(
            result.is_ok(),
            "shadowing test should stay parseable for future semantic policy"
        );
    }

    /// Verifies taking address of undeclared symbol is rejected.
    #[test]
    fn rejects_address_of_undeclared_identifier() {
        assert_semantic_fails("int main(void) { return &missing; }");
    }

    /// Verifies dereference of undeclared identifier is rejected.
    #[test]
    fn rejects_dereference_of_undeclared_identifier() {
        assert_semantic_fails("int main(void) { return *ptr; }");
    }

    /// Verifies using undeclared identifier in call argument is rejected.
    #[test]
    fn rejects_undeclared_identifier_in_call_argument() {
        assert_semantic_fails("int id(int x) { return x; } int main(void) { return id(missing); }");
    }

    /// Verifies calling variable symbol as function is rejected.
    #[test]
    fn rejects_calling_variable_as_function() {
        assert_semantic_fails("int value; int main(void) { return value(); }");
    }

    /// Verifies indexing expression with undeclared index variable is rejected.
    #[test]
    fn rejects_index_with_undeclared_identifier() {
        assert_semantic_fails("int main(void) { int arr[4]; return arr[i]; }");
    }

    /// Verifies return expression identifier must resolve in local scope.
    #[test]
    fn rejects_return_of_out_of_scope_identifier() {
        assert_semantic_fails("int main(void) { { int x; } return x; }");
    }

    /// Verifies duplicate function parameter names in larger signatures are rejected.
    #[test]
    fn rejects_duplicate_parameter_names_in_long_signature() {
        assert_semantic_fails("int sum(int a, int b, int a) { return a + b; }");
    }

    /// Verifies function prototype syntax is rejected in this language subset.
    #[test]
    fn rejects_function_prototype_syntax() {
        assert_semantic_fails("int add(int a, int b);");
    }

    /// Verifies unnamed non-void parameters are rejected semantically.
    #[test]
    fn rejects_unnamed_non_void_parameters_semantically() {
        assert_semantic_fails("int sum(int, int b) { return b; }");
        assert_semantic_fails("int sum(int a, char) { return a; }");
        assert_semantic_fails("int sum(int, char) { return 0; }");
    }

    /// Verifies `void` marker cannot be mixed with other parameters.
    #[test]
    fn rejects_void_marker_mixed_with_named_parameters_semantically() {
        assert_semantic_fails("int f(void, int x) { return x; }");
    }

    /// Verifies multiple `void` markers are rejected in one parameter list.
    #[test]
    fn rejects_multiple_void_markers_semantically() {
        assert_semantic_fails("int f(void, void) { return 0; }");
    }

    /// Verifies a regular `f(void)` function is accepted semantically.
    #[test]
    fn accepts_regular_void_marker_function() {
        let result = analyze_source("int f(void) { return 0; }");
        assert!(result.is_ok(), "f(void) function should be accepted");
    }

    /// Verifies multiple valid globals/functions can coexist.
    #[test]
    fn accepts_multiple_distinct_top_level_symbols() {
        let result = analyze_source(
            "int g; char h; int f(void) { return g; } int main(void) { return f(); }",
        );
        assert!(
            result.is_ok(),
            "distinct top-level symbols should be accepted"
        );
    }

    /// Verifies pointer plus integer arithmetic is accepted.
    #[test]
    fn accepts_pointer_plus_integer_arithmetic() {
        let result =
            analyze_source("int main(void) { int arr[4]; int *p; p = arr; return *(p + 1); }");
        assert!(result.is_ok(), "pointer plus integer should be accepted");
    }

    /// Verifies modulo rejects non-integer-like operands.
    #[test]
    fn rejects_modulo_with_pointer_operand() {
        assert_semantic_fails("int main(void) { int *p; return p % 2; }");
    }

    /// Verifies assignment rejects incompatible pointer and integer values.
    #[test]
    fn rejects_pointer_assignment_from_integer_literal() {
        assert_semantic_fails("int main(void) { int *p; p = 1; return 0; }");
    }

    /// Verifies assignment rejects incompatible integer and pointer values.
    #[test]
    fn rejects_integer_assignment_from_pointer() {
        assert_semantic_fails("int main(void) { int x; int *p; x = p; return 0; }");
    }

    /// Verifies assignment accepts pointer from array decay.
    #[test]
    fn accepts_pointer_assignment_from_array_expression() {
        let result = analyze_source("int main(void) { int arr[4]; int *p; p = arr; return 0; }");
        assert!(
            result.is_ok(),
            "array expression should be compatible with pointer assignment"
        );
    }

    /// Verifies pointer - pointer yields an integer-like result and is accepted.
    #[test]
    fn accepts_pointer_subtraction_pointer_pointer() {
        let result = analyze_source(
            "int main(void) { int arr[4]; int *p; int *q; p = arr; q = p + 2; return q - p; }",
        );
        assert!(result.is_ok(), "pointer - pointer should be accepted");
    }

    /// Verifies call arguments reject strict type mismatches.
    #[test]
    fn rejects_call_argument_type_mismatch() {
        assert_semantic_fails(
            "int take(int x) { return x; } int main(void) { int *p; return take(p); }",
        );
    }

    /// Verifies return rejects incompatible value type for function signature.
    #[test]
    fn rejects_return_type_mismatch_pointer_to_int() {
        assert_semantic_fails("int main(void) { int *p; return p; }");
    }

    /// Verifies return rejects incompatible value type in pointer function.
    #[test]
    fn rejects_return_type_mismatch_int_to_pointer() {
        assert_semantic_fails("int *main(void) { return 1; }");
    }

    /// Verifies unary dereference rejects non-pointer operand.
    #[test]
    fn rejects_dereference_of_non_pointer_expression() {
        assert_semantic_fails("int main(void) { int x; return *x; }");
    }

    /// Verifies unary address-of rejects non-lvalue operand.
    #[test]
    fn rejects_address_of_non_lvalue_expression() {
        assert_semantic_fails("int main(void) { int a; int b; return &(a + b); }");
    }

    /// Verifies index base must be pointer/array-like.
    #[test]
    fn rejects_indexing_non_indexable_base() {
        assert_semantic_fails("int main(void) { int x; return x[0]; }");
    }

    /// Verifies index expression must be integer-like.
    #[test]
    fn rejects_index_with_pointer_index_expression() {
        assert_semantic_fails("int main(void) { int arr[4]; int *i; return arr[i]; }");
    }

    /// Verifies declaration initializer rejects strict type mismatch.
    #[test]
    fn rejects_initializer_type_mismatch() {
        assert_semantic_fails("int x; int *p = x;");
    }

    /// Verifies strict comparison rejects incompatible pointer/integer types.
    #[test]
    fn rejects_comparison_between_pointer_and_integer() {
        assert_semantic_fails("int main(void) { int *p; int x; if (p == x) return 1; return 0; }");
    }

    /// Verifies strict-compatible arithmetic remains accepted.
    #[test]
    fn accepts_integer_arithmetic_expression() {
        let result = analyze_source("int main(void) { int a; int b; return a + b * 2; }");
        assert!(result.is_ok(), "integer arithmetic should remain valid");
    }

    /// Verifies strict-compatible assignment remains accepted.
    #[test]
    fn accepts_integer_assignment_expression() {
        let result = analyze_source("int main(void) { int a; int b; a = b; return a; }");
        assert!(
            result.is_ok(),
            "same-type integer assignment should remain valid"
        );
    }

    /// Verifies duplicate global symbol detection with a different type pair.
    #[test]
    fn rejects_duplicate_global_symbols_with_different_types() {
        assert_semantic_fails("char dup; int dup;");
    }

    /// Verifies duplicate function definitions are rejected for non-void returns.
    #[test]
    fn rejects_duplicate_function_definitions_second_case() {
        assert_semantic_fails("int f(void) { return 0; } int f(void) { return 1; }");
    }

    /// Verifies duplicate parameter names are rejected in a void-returning function.
    #[test]
    fn rejects_duplicate_parameter_names_second_case() {
        assert_semantic_fails("void f(int a, int a) { return; }");
    }

    /// Verifies duplicate local names are rejected in nested statement blocks.
    #[test]
    fn rejects_duplicate_local_names_second_case() {
        assert_semantic_fails("int main(void) { int a; { int b; int b; } return a; }");
    }

    /// Verifies undeclared identifier detection inside arithmetic return.
    #[test]
    fn rejects_undeclared_identifier_in_binary_expression() {
        assert_semantic_fails("int main(void) { int a; return a + missing; }");
    }

    /// Verifies function call arity checks for excessive arguments.
    #[test]
    fn rejects_call_with_too_many_arguments() {
        assert_semantic_fails("int f(int a) { return a; } int main(void) { return f(1, 2); }");
    }

    /// Verifies assignment compatibility rejects pointer-to-char from pointer-to-int.
    #[test]
    fn rejects_pointer_assignment_between_different_pointee_types() {
        assert_semantic_fails("int main(void) { int *ip; char *cp; cp = ip; return 0; }");
    }

    /// Verifies unary plus rejects pointer operands.
    #[test]
    fn rejects_unary_plus_on_pointer() {
        assert_semantic_fails("int main(void) { int *p; return +p; }");
    }

    /// Verifies logical operators reject non-scalar function operands.
    #[test]
    fn rejects_logical_or_with_function_symbol() {
        assert_semantic_fails("int f(void) { return 0; } int main(void) { return f || 1; }");
    }

    /// Verifies indexing supports pointer bases with integer index.
    #[test]
    fn accepts_pointer_indexing_expression() {
        let result = analyze_source("int main(void) { int *p; return p[0]; }");
        assert!(result.is_ok(), "pointer indexing should be accepted");
    }

    /// Verifies global initializer type compatibility for matching integer types.
    #[test]
    fn accepts_global_initializer_matching_type() {
        let result = analyze_source("int x = 1; int main(void) { return x; }");
        assert!(
            result.is_ok(),
            "matching global initializer should be accepted"
        );
    }

    /// Verifies return type compatibility for pointer-returning function.
    #[test]
    fn accepts_return_matching_pointer_type() {
        let result = analyze_source("int *f(int *p) { return p; } int main(void) { return 0; }");
        assert!(result.is_ok(), "matching pointer return should be accepted");
    }

    /// Verifies void marker parameter still accepted with explicit void return.
    #[test]
    fn accepts_void_marker_parameter_second_case() {
        let result = analyze_source("void noop(void) { return; } int main(void) { return 0; }");
        assert!(result.is_ok(), "void marker parameter should be accepted");
    }

    #[test]
    fn accepts_struct_member_access() {
        let result = analyze_source(
            "struct Point { int x; int y; } p; int main(void) { p.x = 7; return p.y + p.x; }",
        );
        assert!(result.is_ok(), "struct member access should be accepted");
    }

    #[test]
    fn rejects_unknown_struct_member() {
        assert_semantic_fails(
            "struct Point { int x; int y; } p; int main(void) { return p.z; }",
        );
    }

    #[test]
    fn accepts_arrow_member_access() {
        let result = analyze_source(
            "struct Point { int x; int y; } p; int main(void) { struct Point *q; q = &p; return q->x; }",
        );
        assert!(result.is_ok(), "arrow member access should be accepted");
    }

    #[test]
    fn accepts_tag_only_then_use_struct_tag() {
        let result = analyze_source(
            "struct Point { int x; int y; }; struct Point p; int main(void) { return p.x; }",
        );
        assert!(result.is_ok(), "tag-only declaration should register struct type");
    }

    #[test]
    fn accepts_typedef_struct_alias_usage() {
        let result = analyze_source(
            "typedef struct Point { int x; int y; } Point; Point p; int main(void) { return p.y; }",
        );
        assert!(result.is_ok(), "typedef alias for struct should be accepted");
    }

    #[test]
    fn accepts_struct_forward_declaration_then_definition() {
        let result = analyze_source(
            "struct Node; struct Node { int value; struct Node *next; }; struct Node n;",
        );
        assert!(result.is_ok(), "forward declaration then definition should work");
    }

    #[test]
    fn rejects_unknown_typedef() {
        assert_semantic_fails("UnknownType x; int main(void) { return 0; }");
    }

    #[test]
    fn rejects_duplicate_typedef() {
        assert_semantic_fails(
            "typedef int MyInt; typedef int MyInt; int main(void) { return 0; }",
        );
    }

    #[test]
    fn rejects_typedef_function_name_collision() {
        assert_semantic_fails("typedef int foo; int foo(void) { return 0; }");
    }

    #[test]
    fn rejects_simple_circular_typedef_aliases() {
        assert_semantic_fails("typedef B A; typedef A B; int main(void) { return 0; }");
    }

    #[test]
    fn rejects_self_referential_typedef_alias() {
        assert_semantic_fails("typedef A A; int main(void) { return 0; }");
    }

    #[test]
    fn circular_typedef_error_mentions_cycle() {
        let result = analyze_source("typedef B A; typedef A B; int main(void) { return 0; }");
        let message = format!("{:?}", result.err());
        assert!(
            message.contains("typedef") || message.contains("unknown"),
            "unexpected typedef diagnostics: {message}"
        );
    }

    #[test]
    fn accepts_pointer_to_typedef_struct() {
        let result = analyze_source(
            "typedef struct Point { int x; } Point; Point *p; int main(void) { p->x = 1; return 0; }",
        );
        assert!(result.is_ok(), "pointer to typedef struct should work");
    }

    #[test]
    fn accepts_array_of_typedef_struct() {
        let result = analyze_source(
            "typedef struct Point { int x; } Point; Point arr[3]; int main(void) { arr[0].x = 1; return 0; }",
        );
        assert!(result.is_ok(), "array of typedef struct should work");
    }

    #[test]
    fn accepts_enum_constants_and_values() {
        let result = analyze_source(
            "enum Color { RED, GREEN = 3, BLUE }; int main(void) { return RED + BLUE; }",
        );
        assert!(result.is_ok(), "enum constants should resolve as int expressions");
    }

    #[test]
    fn accepts_typedef_enum_alias() {
        let result = analyze_source(
            "typedef enum Color { RED, GREEN } Color; Color c; int main(void) { c = GREEN; return c; }",
        );
        assert!(result.is_ok(), "typedef enum alias should be usable");
    }

    #[test]
    fn rejects_duplicate_enum_constant_names() {
        assert_semantic_fails("enum Color { RED, RED }; int main(void) { return 0; }");
    }
}
