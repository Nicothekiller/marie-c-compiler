/// Root AST node representing a full C translation unit.
#[derive(Debug, Clone, Default)]
pub struct TranslationUnit {
    /// Top-level declarations and definitions in source order.
    pub top_level_items: Vec<TopLevelItem>,
}

/// Placeholder enum for future concrete top-level AST items.
#[derive(Debug, Clone)]
pub enum TopLevelItem {
    /// Temporary variant used during early compiler scaffolding.
    Placeholder,
}
