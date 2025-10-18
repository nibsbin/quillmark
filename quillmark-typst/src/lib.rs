//! # Typst Backend for Quillmark
//!
//! This crate provides a complete Typst backend implementation that converts Markdown
//! documents to PDF and SVG formats via the Typst typesetting system.
//!
//! ## Overview
//!
//! The primary entry point is the [`TypstBackend`] struct, which implements the
//! [`Backend`] trait from `quillmark-core`. Users typically interact with this backend
//! through the high-level `Workflow` API from the `quillmark` crate.
//!
//! ## Features
//!
//! - Converts CommonMark Markdown to Typst markup
//! - Compiles Typst documents to PDF and SVG formats
//! - Provides template filters for YAML data transformation
//! - Manages fonts, assets, and packages dynamically
//! - Thread-safe for concurrent rendering
//!
//! ## Example Usage
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
//! ## Modules
//!
//! - [`convert`] - Markdown to Typst conversion utilities
//! - [`compile`] - Typst to PDF/SVG compilation functions
//!
//! Note: The `error_mapping` module provides internal utilities for converting Typst
//! diagnostics to Quillmark diagnostics and is not part of the public API.

pub mod compile;
pub mod convert;
mod error_mapping;
mod filters;
mod world;

/// Utilities exposed for fuzzing tests.
/// Not intended for general use.
#[doc(hidden)]
pub mod fuzz_utils {
    pub use super::filters::inject_json;
}

use filters::{
    asset_filter, content_filter, date_filter, dict_filter, lines_filter, string_filter,
};
use quillmark_core::{
    Artifact, Backend, Glue, OutputFormat, Quill, RenderError, RenderOptions, RenderResult,
};

/// Typst backend implementation for Quillmark.
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
    ) -> Result<RenderResult, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        // Check if format is supported
        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format,
            });
        }

        match format {
            OutputFormat::Pdf => {
                let bytes = compile::compile_to_pdf(quill, glued_content)?;
                let artifacts = vec![Artifact {
                    bytes,
                    output_format: OutputFormat::Pdf,
                }];
                Ok(RenderResult::new(artifacts, OutputFormat::Pdf))
            }
            OutputFormat::Svg => {
                let svg_pages = compile::compile_to_svg(quill, glued_content)?;
                let artifacts = svg_pages
                    .into_iter()
                    .map(|bytes| Artifact {
                        bytes,
                        output_format: OutputFormat::Svg,
                    })
                    .collect();
                Ok(RenderResult::new(artifacts, OutputFormat::Svg))
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
