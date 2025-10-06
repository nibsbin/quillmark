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
}
