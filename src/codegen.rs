use crate::ast::{BinaryOp, Statement, TranslationUnit};
use crate::error::CompilerError;

mod emitter;
mod validation;

#[cfg(test)]
mod tests;

use emitter::MarieEmitter;
use validation::validate_ast;

/// Backend interface for emitting target assembly from AST.
pub trait Codegen {
    /// Emits target output text from a semantic-validated AST.
    fn emit(&self, ast: &TranslationUnit) -> Result<String, CompilerError>;
}

/// Target-specific validation for unsupported features.
pub trait TargetValidation {
    /// Validates the AST contains only features supported by this target.
    fn validate(&self, _ast: &TranslationUnit) -> Result<(), CompilerError> {
        Ok(())
    }

    /// Binary operations not supported by this target.
    fn unsupported_binary_ops(&self) -> &'static [BinaryOp] {
        &[]
    }

    /// Statement types not supported by this target.
    fn unsupported_statement_kinds(&self) -> &'static [fn() -> Statement] {
        &[]
    }

    /// Checks for unsupported storage classes in declarations.
    fn unsupported_storage_classes(&self) -> &'static [fn() -> crate::ast::StorageClass] {
        &[|| crate::ast::StorageClass::Static]
    }

    /// Checks for unsupported type qualifiers.
    fn unsupported_type_qualifiers(&self) -> &'static [fn() -> crate::ast::Type] {
        &[|| crate::ast::Type::Const(Box::new(crate::ast::Type::Builtin(crate::ast::BuiltinType::Int)))]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarieCodegen;

impl Codegen for MarieCodegen {
    /// Emits Marie assembly text from the provided AST.
    fn emit(&self, ast: &TranslationUnit) -> Result<String, CompilerError> {
        let mut emitter = MarieEmitter::default();
        emitter.emit_translation_unit(ast)?;
        Ok(emitter.finish())
    }
}

impl TargetValidation for MarieCodegen {
    fn unsupported_binary_ops(&self) -> &'static [BinaryOp] {
        &[
            BinaryOp::ShiftLeft,
            BinaryOp::ShiftRight,
            BinaryOp::BitwiseAnd,
            BinaryOp::BitwiseOr,
            BinaryOp::BitwiseXor,
        ]
    }

    fn validate(&self, ast: &TranslationUnit) -> Result<(), CompilerError> {
        validate_ast(
            ast,
            self.unsupported_binary_ops(),
            self.unsupported_statement_kinds(),
            self.unsupported_storage_classes(),
            self.unsupported_type_qualifiers(),
        )
    }
}
