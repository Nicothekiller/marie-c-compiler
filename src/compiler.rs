use crate::codegen::MarieCodegen;
use crate::error::CompilerError;
use crate::parser::CParser;
use crate::semantic::{SemanticAnalyzer, SemanticInfo};

pub struct Compiler {
    parser: CParser,
    semantic: SemanticAnalyzer,
}

impl Compiler {
    /// Creates a new compiler pipeline instance.
    pub fn new() -> Self {
        Self {
            parser: CParser::new(),
            semantic: SemanticAnalyzer::new(),
        }
    }

    /// Compiles preprocessed C source text into Marie assembly output.
    pub fn compile_source(&self, source: &str) -> Result<String, CompilerError> {
        let ast = self.parser.parse_translation_unit(source)?;
        self.semantic.analyze(&ast)?;
        Ok(MarieCodegen::emit(&ast))
    }

    /// Parses and runs semantic analysis, returning intermediate compilation artifacts.
    pub fn frontend(&self, source: &str) -> Result<FrontendArtifacts, CompilerError> {
        let ast = self.parser.parse_translation_unit(source)?;
        let semantic_info = self.semantic.analyze(&ast)?;

        Ok(FrontendArtifacts { ast, semantic_info })
    }
}

/// Frontend outputs produced before code generation.
#[derive(Debug)]
pub struct FrontendArtifacts {
    pub ast: crate::ast::TranslationUnit,
    pub semantic_info: SemanticInfo,
}

impl Default for Compiler {
    /// Creates a default compiler pipeline instance.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Compiler;

    /// Verifies the compiler pipeline produces placeholder Marie output.
    #[test]
    fn compile_source_returns_marie_output() {
        let compiler = Compiler::new();
        let output = compiler
            .compile_source("int main(void) { return 0; }")
            .expect("source should compile in placeholder pipeline");

        assert!(output.contains("HALT"));
    }

    /// Verifies frontend stage returns AST and semantic metadata.
    #[test]
    fn frontend_returns_semantic_artifacts() {
        let compiler = Compiler::new();
        let artifacts = compiler
            .frontend("int main(void) { return 0; }")
            .expect("frontend should succeed");

        assert_eq!(artifacts.ast.top_level_items.len(), 1);
        assert!(
            artifacts
                .semantic_info
                .function_signatures
                .contains_key("main")
        );
    }
}
