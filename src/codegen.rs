use crate::ast::TranslationUnit;
use crate::error::CompilerError;

/// Backend interface for emitting target assembly from AST.
pub trait Codegen {
    /// Emits target output text from a semantic-validated AST.
    fn emit(&self, ast: &TranslationUnit) -> Result<String, CompilerError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarieCodegen;

impl Codegen for MarieCodegen {
    /// Emits Marie assembly text from the provided AST.
    fn emit(&self, _ast: &TranslationUnit) -> Result<String, CompilerError> {
        Ok(["/ marie-c-compiler output (placeholder)", "HALT"].join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::TranslationUnit;

    use super::{Codegen, MarieCodegen};

    /// Confirms the emitter returns a minimal placeholder Marie program.
    #[test]
    fn emits_placeholder_marie_program() {
        let output = MarieCodegen
            .emit(&TranslationUnit::default())
            .expect("codegen should produce placeholder output");

        assert!(output.contains("/ marie-c-compiler output (placeholder)"));
        assert!(output.contains("HALT"));
    }
}
