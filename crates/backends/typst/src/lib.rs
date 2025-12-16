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

/// Embedded default Quill files
mod embedded {
    pub const QUILL_TOML: &str = include_str!("../default_quill/Quill.toml");
    pub const PLATE_TYP: &str = include_str!("../default_quill/plate.typ");
    pub const EXAMPLE_MD: &str = include_str!("../default_quill/example.md");
}

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
    Artifact, Backend, Diagnostic, OutputFormat, Plate, Quill, RenderError, RenderOptions,
    RenderResult, Severity,
};
use std::collections::HashMap;

/// Typst backend implementation for Quillmark.
pub struct TypstBackend;

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        &[OutputFormat::Pdf, OutputFormat::Svg]
    }

    fn plate_extension_types(&self) -> &'static [&'static str] {
        &["typ"]
    }

    fn allow_auto_plate(&self) -> bool {
        true
    }

    fn register_filters(&self, plate: &mut Plate) {
        // Register basic filters (simplified for now)
        plate.register_filter("String", string_filter);
        plate.register_filter("Lines", lines_filter);
        plate.register_filter("Date", date_filter);
        plate.register_filter("Dict", dict_filter);
        plate.register_filter("Content", content_filter);
        plate.register_filter("Asset", asset_filter);
        plate.register_filter("Json", filters::json_filter);
    }

    fn compile(
        &self,
        plated: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        // Check if format is supported
        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!("{:?} not supported by {} backend", format, self.id()),
                    )
                    .with_code("backend::format_not_supported".to_string())
                    .with_hint(format!("Supported formats: {:?}", self.supported_formats())),
                ),
            });
        }

        match format {
            OutputFormat::Pdf => {
                let bytes = compile::compile_to_pdf(quill, plated)?;
                let artifacts = vec![Artifact {
                    bytes,
                    output_format: OutputFormat::Pdf,
                }];
                Ok(RenderResult::new(artifacts, OutputFormat::Pdf))
            }
            OutputFormat::Svg => {
                let svg_pages = compile::compile_to_svg(quill, plated)?;
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
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!("Text output not supported by {} backend", self.id()),
                    )
                    .with_code("backend::format_not_supported".to_string())
                    .with_hint(format!("Supported formats: {:?}", self.supported_formats())),
                ),
            }),
        }
    }

    fn default_quill(&self) -> Option<Quill> {
        use quillmark_core::FileTreeNode;

        // Build file tree from embedded files
        let mut files = HashMap::new();
        files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: embedded::QUILL_TOML.as_bytes().to_vec(),
            },
        );
        files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: embedded::PLATE_TYP.as_bytes().to_vec(),
            },
        );
        files.insert(
            "example.md".to_string(),
            FileTreeNode::File {
                contents: embedded::EXAMPLE_MD.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files };

        // Try to create Quill from tree, return None if it fails
        Quill::from_tree(root, None).ok()
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
        let backend = TypstBackend;
        assert_eq!(backend.id(), "typst");
        assert!(backend.allow_auto_plate());
        assert!(backend.supported_formats().contains(&OutputFormat::Pdf));
        assert!(backend.supported_formats().contains(&OutputFormat::Svg));
    }
}
