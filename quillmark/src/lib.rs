use std::error::Error;

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
    FormatNotSupported { backend: String, format: OutputFormat },
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

/// Render markdown using the specified options
/// 
/// This function will select an appropriate backend based on the options provided.
/// If no backend is specified, it will try to find a suitable one based on the
/// requested output format.
pub fn render(_markdown: &str, _options: &Options) -> RenderResult {
    // For now, return an error indicating no backends are available
    // This will be implemented when backends are registered
    Err(RenderError::UnsupportedBackend("no backends available".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_with_no_backends() {
        let options = Options {
            backend: None,
            format: Some(OutputFormat::Pdf),
        };
        
        let result = render("# Hello World", &options);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            RenderError::UnsupportedBackend(_) => {},
            _ => panic!("Expected UnsupportedBackend error"),
        }
    }

    #[test]
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Pdf, OutputFormat::Pdf);
        assert_ne!(OutputFormat::Pdf, OutputFormat::Svg);
    }
}