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
    UnsupportedFeature(Diagnostic),
}

impl CompilerError {
    /// Creates a parse error with optional source location.
    pub fn parse_with_location(
        message: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self::Parse(Diagnostic {
            message: message.into(),
            location,
        })
    }

    /// Creates a parse error without source location.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::parse_with_location(message, None)
    }

    /// Creates a parse error with source location.
    pub fn parse_at(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::parse_with_location(message, Some(location))
    }

    /// Creates a semantic error with optional source location.
    pub fn semantic_with_location(
        message: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self::Semantic(Diagnostic {
            message: message.into(),
            location,
        })
    }

    /// Creates a semantic error without source location.
    pub fn semantic(message: impl Into<String>) -> Self {
        Self::semantic_with_location(message, None)
    }

    /// Creates a semantic error with source location.
    pub fn semantic_at(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::semantic_with_location(message, Some(location))
    }

    /// Creates an unsupported feature error with optional source location.
    pub fn unsupported_with_location(
        message: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self::UnsupportedFeature(Diagnostic {
            message: message.into(),
            location,
        })
    }

    /// Creates an unsupported feature error without source location.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::unsupported_with_location(message, None)
    }

    /// Creates an unsupported feature error with source location.
    pub fn unsupported_at(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::unsupported_with_location(message, Some(location))
    }
}

impl Display for CompilerError {
    /// Formats the compiler error for user-facing output.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Parse(diagnostic) => format_diagnostic(f, "parse", diagnostic),
            Self::Semantic(diagnostic) => format_diagnostic(f, "semantic", diagnostic),
            Self::UnsupportedFeature(diagnostic) => {
                format_diagnostic(f, "unsupported feature", diagnostic)
            }
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
