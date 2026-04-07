use std::fmt::{Display, Formatter};

/// 1-based source location in input text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// Shared diagnostic payload for parse/semantic errors.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub location: Option<SourceLocation>,
}

#[derive(Debug)]
pub enum CompilerError {
    Io(std::io::Error),
    Parse(Diagnostic),
    Semantic(Diagnostic),
}

impl CompilerError {
    /// Creates a parse error without source location.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse(Diagnostic {
            message: message.into(),
            location: None,
        })
    }

    /// Creates a parse error with source location.
    pub fn parse_at(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::Parse(Diagnostic {
            message: message.into(),
            location: Some(location),
        })
    }

    /// Creates a semantic error without source location.
    pub fn semantic(message: impl Into<String>) -> Self {
        Self::Semantic(Diagnostic {
            message: message.into(),
            location: None,
        })
    }

    /// Creates a semantic error with source location.
    pub fn semantic_at(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::Semantic(Diagnostic {
            message: message.into(),
            location: Some(location),
        })
    }
}

impl Display for CompilerError {
    /// Formats the compiler error for user-facing output.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Parse(diagnostic) => format_diagnostic(f, "parse", diagnostic),
            Self::Semantic(diagnostic) => format_diagnostic(f, "semantic", diagnostic),
        }
    }
}

fn format_diagnostic(
    f: &mut Formatter<'_>,
    category: &str,
    diagnostic: &Diagnostic,
) -> std::fmt::Result {
    if let Some(location) = diagnostic.location {
        write!(
            f,
            "{category} error at line {}, column {}: {}",
            location.line, location.column, diagnostic.message
        )
    } else {
        write!(f, "{category} error: {}", diagnostic.message)
    }
}

impl std::error::Error for CompilerError {}

impl From<std::io::Error> for CompilerError {
    /// Converts an I/O error into a compiler error.
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
