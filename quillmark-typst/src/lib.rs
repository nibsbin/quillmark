//! Typst backend for Quillmark document rendering.
//!
//! This crate provides a complete Typst backend implementation that converts Markdown
//! documents to PDF and SVG formats via the Typst typesetting system.
//!
//! # Overview
//!
//! The primary entry point is the [`TypstBackend`] struct, which implements the
//! [`Backend`] trait from `quillmark-core`. Users typically interact with this backend
//! through the high-level `Workflow` API from the `quillmark` crate.
//!
//! # Features
//!
//! - Converts CommonMark Markdown to Typst markup
//! - Compiles Typst documents to PDF and SVG formats
//! - Provides template filters for YAML data transformation
//! - Manages fonts, assets, and packages dynamically
//! - Thread-safe for concurrent rendering
//!
//! # Example
//!
//! ```no_run
//! use quillmark_typst::TypstBackend;
//! use quillmark_core::{Backend, Quill, OutputFormat};
//!
//! let backend = TypstBackend::default();
//! let quill = Quill::from_path("path/to/quill").unwrap();
//!
//! // Use with Workflow API (recommended)
//! // let workflow = Workflow::new(Box::new(backend), quill);
//! ```
//!
//! # Documentation
//!
//! For detailed API documentation, see [API.md](https://github.com/nibsbin/quillmark/blob/main/quillmark-typst/API.md).
//!
//! For Markdown to Typst conversion details, see [CONVERT.md](https://github.com/nibsbin/quillmark/blob/main/quillmark-typst/CONVERT.md).

pub mod compile;
pub mod convert;
mod filters;
mod world;
use filters::{
    asset_filter, content_filter, date_filter, dict_filter, lines_filter, string_filter,
};
use quillmark_core::{Artifact, Backend, Glue, OutputFormat, Quill, RenderError, RenderOptions};

/// Typst backend implementation for Quillmark.
///
/// This struct implements the [`Backend`] trait to provide Typst rendering capabilities.
/// It supports compilation to PDF and SVG formats.
///
/// # Supported Formats
///
/// - [`OutputFormat::Pdf`] - Portable Document Format
/// - [`OutputFormat::Svg`] - Scalable Vector Graphics (one file per page)
///
/// # Template Filters
///
/// The backend registers the following filters for use in Typst templates:
///
/// - `String` - Converts values to Typst string literals
/// - `Lines` - Converts arrays to Typst arrays
/// - `Date` - Converts ISO 8601 dates to Typst datetime objects
/// - `Dict` - Converts YAML/JSON objects to Typst dictionaries
/// - `Content` - Converts Markdown to Typst markup
/// - `Asset` - Resolves asset paths for Typst
///
/// # Examples
///
/// ```
/// use quillmark_typst::TypstBackend;
/// use quillmark_core::Backend;
///
/// let backend = TypstBackend::default();
/// assert_eq!(backend.id(), "typst");
/// ```
pub struct TypstBackend;

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        &[OutputFormat::Pdf, OutputFormat::Svg]
    }

    fn glue_type(&self) -> &'static str {
        ".typ"
    }

    fn register_filters(&self, glue: &mut Glue) {
        // Register basic filters (simplified for now)
        glue.register_filter("String", string_filter);
        glue.register_filter("Lines", lines_filter);
        glue.register_filter("Date", date_filter);
        glue.register_filter("Dict", dict_filter);
        glue.register_filter("Content", content_filter);
        glue.register_filter("Asset", asset_filter);
    }

    fn compile(
        &self,
        glued_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<Vec<Artifact>, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        // Check if format is supported
        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format,
            });
        }

        println!("Typst backend compiling for quill: {}", quill.name);

        match format {
            OutputFormat::Pdf => {
                let bytes = compile::compile_to_pdf(quill, glued_content).unwrap();
                Ok(vec![Artifact {
                    bytes: bytes,
                    output_format: OutputFormat::Pdf,
                }])
            }
            OutputFormat::Svg => {
                let svg_pages = compile::compile_to_svg(quill, glued_content).map_err(|e| {
                    RenderError::Other(format!("SVG compilation failed: {}", e).into())
                })?;
                Ok(svg_pages
                    .into_iter()
                    .map(|bytes| Artifact {
                        bytes,
                        output_format: OutputFormat::Svg,
                    })
                    .collect())
            }
            OutputFormat::Txt => Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format: OutputFormat::Txt,
            }),
        }
    }
}

impl Default for TypstBackend {
    /// Creates a new [`TypstBackend`] instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use quillmark_typst::TypstBackend;
    ///
    /// let backend = TypstBackend::default();
    /// ```
    fn default() -> Self {
        Self
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_info() {
        let backend = TypstBackend::default();
        assert_eq!(backend.id(), "typst");
        assert_eq!(backend.glue_type(), ".typ");
        assert!(backend.supported_formats().contains(&OutputFormat::Pdf));
        assert!(backend.supported_formats().contains(&OutputFormat::Svg));
    }
}

// Re-export for compatibility
pub use TypstBackend as backend;
