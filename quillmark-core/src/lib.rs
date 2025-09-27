use std::error::Error;
use std::path::PathBuf;

// Re-export parsing functionality
pub mod parse;
pub use parse::{decompose, ParsedDocument, BODY_FIELD};

/// Output formats supported by backends
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Txt,
    Svg,
    Pdf,
}

/// An artifact produced by rendering
#[derive(Debug)]
pub struct Artifact {
    pub bytes: Vec<u8>,
    pub output_format: OutputFormat,
}

/// Rendering options
pub struct Options {
    pub backend: Option<String>, // Backend identifier, not the trait object itself
    pub format: Option<OutputFormat>,
}

/// Result type for rendering operations
pub type RenderResult = Result<Vec<Artifact>, RenderError>;

/// Errors that can occur during rendering
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("{0:?} backend is not built in this binary")]
    UnsupportedBackend(String),
    #[error("{format:?} not supported by {backend:?}")]
    FormatNotSupported {
        backend: String,
        format: OutputFormat,
    },
    #[error("multiple backends can produce {0:?}; specify one explicitly")]
    AmbiguousBackend(OutputFormat),
    #[error(transparent)]
    Other(#[from] Box<dyn Error + Send + Sync>),
}

/// Trait for markdown rendering backends
pub trait Backend: Send + Sync {
    /// Stable identifier (e.g., "typst", "latex", "mock")
    fn id(&self) -> &'static str;

    /// Formats this backend supports in *this* build.
    fn supported_formats(&self) -> &'static [OutputFormat];

    /// Render markdown into one or more artifacts (pages, files, etc.)
    fn render(&self, markdown: &str, opts: &Options) -> Result<Vec<Artifact>, RenderError>;
}

/// Test context helpers for examples and testing
pub mod test_context {
    use super::*;
    
    /// Find the workspace root examples directory
    /// This helper searches for the examples/ folder starting from the current directory
    /// and walking up the directory tree until it finds a Cargo.toml at workspace level.
    pub fn examples_dir() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let current_dir = std::env::current_dir()?;
        let mut dir = current_dir.as_path();
        
        // Walk up the directory tree to find workspace root
        loop {
            let cargo_toml = dir.join("Cargo.toml");
            let examples_dir = dir.join("examples");
            
            // Check if this looks like the workspace root (has both Cargo.toml and examples/)
            if cargo_toml.exists() && examples_dir.exists() {
                // Also check if Cargo.toml contains workspace members to confirm it's the workspace root
                if let Ok(cargo_content) = std::fs::read_to_string(&cargo_toml) {
                    if cargo_content.contains("[workspace]") || cargo_content.contains("members") {
                        return Ok(examples_dir);
                    }
                }
                // Fallback: if we have examples/ directory, use it
                return Ok(examples_dir);
            }
            
            // Move up one directory
            if let Some(parent) = dir.parent() {
                dir = parent;
            } else {
                break;
            }
        }
        
        // If we can't find it, create examples/ in current directory
        let fallback_examples = current_dir.join("examples");
        std::fs::create_dir_all(&fallback_examples)?;
        Ok(fallback_examples)
    }
    
    /// Create an output directory within the examples folder
    /// This ensures all example outputs are staged within the workspace examples folder
    pub fn create_output_dir(subdir: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let examples_root = examples_dir()?;
        let output_dir = examples_root.join(subdir);
        std::fs::create_dir_all(&output_dir)?;
        Ok(output_dir)
    }
    
    /// Get a path to a file within the examples directory
    pub fn examples_path(relative_path: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let examples_root = examples_dir()?;
        Ok(examples_root.join(relative_path))
    }
}
