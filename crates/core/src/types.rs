//! Core types for rendering and output formats.

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
