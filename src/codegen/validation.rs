use crate::ast::{
    BinaryOp, Block, BlockItem, Expression, ExternalDeclaration, Statement, TranslationUnit,
};
use crate::error::CompilerError;

pub(crate) fn validate_ast(
    ast: &TranslationUnit,
    unsupported_ops: &[BinaryOp],
    _unsupported_stmts: &[fn() -> Statement],
    unsupported_storage: &[fn() -> crate::ast::StorageClass],
) -> Result<(), CompilerError> {
    for item in &ast.top_level_items {
        match item {
            ExternalDeclaration::Function(f) => validate_block(&f.body, unsupported_ops)?,
            ExternalDeclaration::GlobalDeclaration(d) => {
                if let Some(sc) = &d.storage_class {
                    for unsupported in unsupported_storage {
                        if *sc == unsupported() {
                            return Err(CompilerError::unsupported_at(
                                "static storage class not supported by target",
                                crate::error::SourceLocation { line: 1, column: 1 },
                            ));
                        }
                    }
                }
                for decl in &d.declarators {
                    if let Some(init) = &decl.initializer {
                        validate_expression(init, unsupported_ops)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_block(block: &Block, unsupported_ops: &[BinaryOp]) -> Result<(), CompilerError> {
    for item in &block.items {
        match item {
            BlockItem::Declaration(d) => {
                for decl in &d.declarators {
                    if let Some(init) = &decl.initializer {
                        validate_expression(init, unsupported_ops)?;
                    }
                }
            }
            BlockItem::Statement(s) => validate_statement(s, unsupported_ops)?,
        }
    }
    Ok(())
}

fn validate_statement(stmt: &Statement, unsupported_ops: &[BinaryOp]) -> Result<(), CompilerError> {
    match stmt {
        Statement::Block(b) => validate_block(b, unsupported_ops)?,
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            validate_expression(condition, unsupported_ops)?;
            validate_statement(then_branch, unsupported_ops)?;
            if let Some(else_b) = else_branch {
                validate_statement(else_b, unsupported_ops)?;
            }
        }
        Statement::While { condition, body } => {
            validate_expression(condition, unsupported_ops)?;
            validate_statement(body, unsupported_ops)?;
        }
        Statement::For {
            init,
            condition,
            update,
            body,
        } => {
            if let Some(i) = init {
                validate_expression(i, unsupported_ops)?;
            }
            if let Some(c) = condition {
                validate_expression(c, unsupported_ops)?;
            }
            if let Some(u) = update {
                validate_expression(u, unsupported_ops)?;
            }
            validate_statement(body, unsupported_ops)?;
        }
        Statement::Return(e) => {
            if let Some(expr) = e {
                validate_expression(expr, unsupported_ops)?;
            }
        }
        Statement::Expression(e) => {
            if let Some(expr) = e {
                validate_expression(expr, unsupported_ops)?;
            }
        }
        Statement::InlineAsm(_) => {}
    }
    Ok(())
}

fn validate_expression(
    expr: &Expression,
    unsupported_ops: &[BinaryOp],
) -> Result<(), CompilerError> {
    match expr {
        Expression::Binary {
            op,
            lhs,
            rhs,
            location,
        } => {
            if unsupported_ops.contains(op) {
                return Err(CompilerError::unsupported_with_location(
                    format!("operator {:?} not supported by target", op),
                    *location,
                ));
            }
            validate_expression(lhs, unsupported_ops)?;
            validate_expression(rhs, unsupported_ops)?;
        }
        Expression::Unary { expr: e, .. } => validate_expression(e, unsupported_ops)?,
        Expression::Assignment { target, value, .. } => {
            validate_expression(target, unsupported_ops)?;
            validate_expression(value, unsupported_ops)?;
        }
        Expression::Call { callee, args, .. } => {
            validate_expression(callee, unsupported_ops)?;
            for arg in args {
                validate_expression(arg, unsupported_ops)?;
            }
        }
        Expression::Index { base, index, .. } => {
            validate_expression(base, unsupported_ops)?;
            validate_expression(index, unsupported_ops)?;
        }
        Expression::ArrayInitializer { elements, .. } => {
            for elem in elements {
                validate_expression(elem, unsupported_ops)?;
            }
        }
        Expression::Identifier { .. } | Expression::IntegerLiteral { .. } => {}
    }
    Ok(())
}
