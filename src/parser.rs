use pest::Parser;
use pest_derive::Parser;

use crate::ast::TranslationUnit;
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
        PestGeneratedParser::parse(Rule::translation_unit, source)
            .map_err(|error| CompilerError::Parse(error.to_string()))?;

        Ok(TranslationUnit::default())
    }
}

#[cfg(test)]
mod tests {
    use super::CParser;

    /// Ensures the parser accepts a basic C-like source snippet.
    #[test]
    fn parses_basic_source_text() {
        let parser = CParser::new();
        let result = parser.parse_translation_unit("int main(void) { return 0; }");

        assert!(result.is_ok());
    }

    /// Ensures the placeholder grammar accepts empty input.
    #[test]
    fn parses_empty_source() {
        let parser = CParser::new();
        let result = parser.parse_translation_unit("");

        assert!(result.is_ok());
    }
}
