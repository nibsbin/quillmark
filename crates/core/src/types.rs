//! Core types for rendering and output formats.
use std::any::Any;

/// Output formats supported by backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum OutputFormat {
    /// Plain text output
    Txt,
    /// Scalable Vector Graphics output
    Svg,
    /// Portable Document Format output
    Pdf,
    /// Portable Network Graphics output (raster)
    Png,
}

/// An artifact produced by rendering.
#[derive(Debug)]
pub struct Artifact {
    /// The binary content of the artifact
    pub bytes: Vec<u8>,
    /// The format of the output
    pub output_format: OutputFormat,
}

/// Internal rendering options.
#[derive(Debug)]
pub struct RenderOptions {
    /// Optional output format specification
    pub output_format: Option<OutputFormat>,
    /// Pixels per inch for raster output formats (e.g., PNG).
    /// Ignored for vector/document formats (PDF, SVG, TXT).
    /// Defaults to 144.0 (2x at 72pt/inch) when `None`.
    pub ppi: Option<f32>,
}

/// Opaque compiled document handle produced by backends that support
/// split compile/render workflows.
pub struct CompiledDocument {
    inner: Box<dyn Any + Send + Sync>,
    /// Number of pages in the compiled document.
    pub page_count: usize,
}

impl CompiledDocument {
    /// Create a new compiled document wrapper.
    pub fn new(inner: Box<dyn Any + Send + Sync>, page_count: usize) -> Self {
        Self { inner, page_count }
    }

    /// Access the opaque backend-specific payload.
    pub fn as_any(&self) -> &(dyn Any + Send + Sync) {
        self.inner.as_ref()
    }
}

impl std::fmt::Debug for CompiledDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledDocument")
            .field("page_count", &self.page_count)
            .finish_non_exhaustive()
    }
}
