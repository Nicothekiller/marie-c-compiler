use crate::codegen::MarieCodegen;
use crate::error::CompilerError;
use crate::parser::CParser;

pub struct Compiler {
    parser: CParser,
}

impl Compiler {
    /// Creates a new compiler pipeline instance.
    pub fn new() -> Self {
        Self {
            parser: CParser::new(),
        }
    }

    /// Compiles preprocessed C source text into Marie assembly output.
    pub fn compile_source(&self, source: &str) -> Result<String, CompilerError> {
        let ast = self.parser.parse_translation_unit(source)?;
        Ok(MarieCodegen::emit(&ast))
    }
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
}
