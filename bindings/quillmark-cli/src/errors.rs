use std::fmt;

/// CLI-specific error type that wraps underlying errors
/// and provides user-friendly error messages
#[derive(Debug)]
pub enum CliError {
    Io(std::io::Error),
    Quillmark(anyhow::Error),
    InvalidArgument(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "I/O error: {}", e),
            CliError::Quillmark(e) => write!(f, "Rendering error: {}", e),
            CliError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::Io(err)
    }
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> Self {
        CliError::Quillmark(err)
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
