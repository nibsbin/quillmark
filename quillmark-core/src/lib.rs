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
