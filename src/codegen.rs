use crate::ast::TranslationUnit;

pub struct MarieCodegen;

impl MarieCodegen {
    /// Emits Marie assembly text from the provided AST.
    pub fn emit(_ast: &TranslationUnit) -> String {
        ["/ marie-c-compiler output (placeholder)", "HALT"].join("\n")
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::TranslationUnit;

    use super::MarieCodegen;

    /// Confirms the emitter returns a minimal placeholder Marie program.
    #[test]
    fn emits_placeholder_marie_program() {
        let output = MarieCodegen::emit(&TranslationUnit::default());

        assert!(output.contains("/ marie-c-compiler output (placeholder)"));
        assert!(output.contains("HALT"));
    }
}
