use crate::codegen::{Codegen, MarieCodegen, TargetValidation};
use crate::error::CompilerError;
use crate::parser::CParser;
use crate::semantic::{SemanticAnalyzer, SemanticInfo};

pub struct Compiler<C: Codegen> {
    parser: CParser,
    semantic: SemanticAnalyzer,
    codegen: C,
}

pub type DefaultCompiler = Compiler<MarieCodegen>;

impl<C: Codegen + TargetValidation> Compiler<C> {
    /// Creates a compiler pipeline instance with an explicit backend.
    pub fn new_with_codegen(codegen: C) -> Self {
        Self {
            parser: CParser::new(),
            semantic: SemanticAnalyzer::new(),
            codegen,
        }
    }

    /// Compiles preprocessed C source text into target backend output.
    pub fn compile_source(&self, source: &str) -> Result<String, CompilerError> {
        let ast = self.parser.parse_translation_unit(source)?;
        self.semantic.analyze(&ast)?;
        self.codegen.validate(&ast)?;
        self.codegen.emit(&ast)
    }

    /// Parses and runs semantic analysis, returning intermediate compilation artifacts.
    pub fn frontend(&self, source: &str) -> Result<FrontendArtifacts, CompilerError> {
        let ast = self.parser.parse_translation_unit(source)?;
        let semantic_info = self.semantic.analyze(&ast)?;

        Ok(FrontendArtifacts { ast, semantic_info })
    }
}

impl<C: Codegen> Compiler<C> {
    /// Creates a compiler pipeline instance without target validation (for backends that don't implement TargetValidation).
    pub fn new_with_codegen_no_validation(codegen: C) -> Self {
        Self {
            parser: CParser::new(),
            semantic: SemanticAnalyzer::new(),
            codegen,
        }
    }

    /// Compiles without target validation.
    pub fn compile_source_no_validation(&self, source: &str) -> Result<String, CompilerError> {
        let ast = self.parser.parse_translation_unit(source)?;
        self.semantic.analyze(&ast)?;
        self.codegen.emit(&ast)
    }
}

impl DefaultCompiler {
    /// Creates a new compiler pipeline instance using the Marie backend.
    pub fn new() -> Self {
        Self::new_with_codegen(MarieCodegen)
    }
}

/// Frontend outputs produced before code generation.
#[derive(Debug)]
pub struct FrontendArtifacts {
    pub ast: crate::ast::TranslationUnit,
    pub semantic_info: SemanticInfo,
}

impl Default for DefaultCompiler {
    /// Creates a default compiler pipeline instance.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{BinaryOp, TranslationUnit};
    use crate::codegen::{Codegen, MarieCodegen, TargetValidation};
    use crate::error::CompilerError;

    use super::{Compiler, DefaultCompiler};

    #[derive(Debug)]
    struct TestCodegen;

    impl Codegen for TestCodegen {
        fn emit(&self, _ast: &TranslationUnit) -> Result<String, CompilerError> {
            Ok("TEST_BACKEND_OUTPUT".to_string())
        }
    }

    impl TargetValidation for TestCodegen {
        fn unsupported_binary_ops(&self) -> &'static [BinaryOp] {
            &[]
        }
    }

    /// Verifies the compiler pipeline produces placeholder Marie output.
    #[test]
    fn compile_source_returns_marie_output() {
        let compiler = DefaultCompiler::new();
        let output = compiler
            .compile_source("int main(void) { return 0; }")
            .expect("source should compile in placeholder pipeline");

        assert!(output.contains("Halt"));
    }

    /// Verifies frontend stage returns AST and semantic metadata.
    #[test]
    fn frontend_returns_semantic_artifacts() {
        let compiler = DefaultCompiler::new();
        let artifacts = compiler
            .frontend("int main(void) { return 0; }")
            .expect("frontend should succeed");

        assert_eq!(artifacts.ast.top_level_items.len(), 1);
        assert!(artifacts
            .semantic_info
            .function_signatures
            .contains_key("main"));
    }

    /// Verifies compiler can use an injected non-Marie backend.
    #[test]
    fn compile_source_uses_injected_codegen_backend() {
        let compiler = Compiler::new_with_codegen(TestCodegen);
        let output = compiler
            .compile_source("int main(void) { return 0; }")
            .expect("source should compile with test backend");

        assert_eq!(output, "TEST_BACKEND_OUTPUT");
    }

    /// Verifies default backend alias remains Marie codegen.
    #[test]
    fn default_compiler_alias_uses_marie_backend_type() {
        let _compiler: Compiler<MarieCodegen> = DefaultCompiler::new();
    }
}
