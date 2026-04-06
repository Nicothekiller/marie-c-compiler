use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum CompilerError {
    Io(std::io::Error),
    Parse(String),
    Semantic(String),
}

impl Display for CompilerError {
    /// Formats the compiler error for user-facing output.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Parse(error) => write!(f, "parse error: {error}"),
            Self::Semantic(error) => write!(f, "semantic error: {error}"),
        }
    }
}

impl std::error::Error for CompilerError {}

impl From<std::io::Error> for CompilerError {
    /// Converts an I/O error into a compiler error.
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
