//! # Backend Trait
//!
//! Backend trait for implementing output format backends.
//!
//! ## Overview
//!
//! The [`Backend`] trait defines the interface that backends must implement
//! to support different output formats (PDF, SVG, TXT, etc.).
//!
//! ## Trait Definition
//!
//! ```rust,ignore
//! pub trait Backend: Send + Sync {
//!     fn id(&self) -> &'static str;
//!     fn supported_formats(&self) -> &'static [OutputFormat];
//!     fn plate_extension_types(&self) -> &'static [&'static str];
//!     fn allow_auto_plate(&self) -> bool;
//!     fn compile(
//!         &self,
//!         plated: &str,
//!         quill: &Quill,
//!         opts: &RenderOptions,
//!     ) -> Result<RenderResult, RenderError>;
//! }
//! ```
//!
//! ## Implementation Guide
//!
//! ### Required Methods
//!
//! #### `id()`
//! Return a unique backend identifier (e.g., "typst", "latex").
//!
//! #### `supported_formats()`
//! Return a slice of [`OutputFormat`] variants this backend supports.
//!
//! #### `plate_extension_types()`
//! Return the file extensions for plate files (e.g., &[".typ"], &[".tex"]).
//! Return an empty array to disable custom plate files.
//!
//! #### `allow_auto_plate()`
//! Return whether automatic JSON plate generation is allowed.
//!
//! #### `compile()`
//! Compile plated content into final artifacts.
//!
//! ```no_run
//! # use quillmark_core::{Quill, RenderOptions, Artifact, OutputFormat, RenderError, RenderResult};
//! # struct MyBackend;
//! # impl MyBackend {
//! fn compile(
//!     &self,
//!     plated: &str,
//!     quill: &Quill,
//!     opts: &RenderOptions,
//! ) -> Result<RenderResult, RenderError> {
//!     // 1. Create compilation environment
//!     // 2. Load assets from quill
//!     // 3. Compile plated content
//!     // 4. Handle errors and map to Diagnostics
//!     // 5. Return RenderResult with artifacts and output format
//!     # let compiled_pdf = vec![];
//!     # let format = OutputFormat::Pdf;
//!     
//!     let artifacts = vec![Artifact {
//!         bytes: compiled_pdf,
//!         output_format: format,
//!     }];
//!     
//!     Ok(RenderResult::new(artifacts, format))
//! }
//! # }
//! ```
//!
//! ## Example Implementation
//!
//! See `backends/quillmark-typst` for a complete backend implementation example.
//!
//! ## Thread Safety
//!
//! The [`Backend`] trait requires `Send + Sync` to enable concurrent rendering.
//! All backend implementations must be thread-safe.

use crate::error::RenderError;
use crate::value::QuillValue;
use crate::{CompiledDocument, Diagnostic, OutputFormat, Quill, RenderOptions, Severity};
use std::collections::HashMap;

/// Backend trait for rendering different output formats
pub trait Backend: Send + Sync + std::fmt::Debug {
    /// Get the backend identifier (e.g., "typst", "latex")
    fn id(&self) -> &'static str;

    /// Get supported output formats
    fn supported_formats(&self) -> &'static [OutputFormat];

    /// Get the plate file extensions accepted by this backend (e.g., &[".typ", ".tex"])
    /// Returns an empty array to disable custom plate files.
    fn plate_extension_types(&self) -> &'static [&'static str];

    /// Compile the plate content with JSON data into final artifacts.
    ///
    /// # Arguments
    ///
    /// * `plate_content` - The plate file content (e.g., Typst source)
    /// * `quill` - The quill template containing assets and configuration
    /// * `opts` - Render options including output format
    /// * `json_data` - JSON value containing the document data
    fn compile(
        &self,
        plate_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
        json_data: &serde_json::Value,
    ) -> Result<crate::RenderResult, RenderError>;

    /// Compile a document to an opaque backend-specific handle for selective page rendering.
    ///
    /// Default implementation returns a "not supported" error so existing backends do not
    /// need to implement this method.
    fn compile_to_document(
        &self,
        _plate_content: &str,
        _quill: &Quill,
        _json_data: &serde_json::Value,
    ) -> Result<CompiledDocument, RenderError> {
        Err(RenderError::UnsupportedBackend {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Backend '{}' does not support compile_to_document()",
                        self.id()
                    ),
                )
                .with_code("backend::compile_to_document_not_supported".to_string()),
            ),
        })
    }

    /// Render selected pages from a previously compiled document.
    ///
    /// - `pages = None` renders all pages in document order.
    /// - `pages = Some(&[])` renders zero pages.
    ///
    /// Default implementation returns a "not supported" error so existing backends do not
    /// need to implement this method.
    fn render_pages(
        &self,
        _doc: &CompiledDocument,
        _pages: Option<&[usize]>,
        _format: OutputFormat,
        _ppi: Option<f32>,
    ) -> Result<crate::RenderResult, RenderError> {
        Err(RenderError::UnsupportedBackend {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!("Backend '{}' does not support render_pages()", self.id()),
                )
                .with_code("backend::render_pages_not_supported".to_string()),
            ),
        })
    }

    /// Transform field values according to backend-specific rules.
    ///
    /// This method is called before JSON serialization to allow backends
    /// to transform field values. For example, the Typst backend converts
    /// markdown fields to Typst markup based on schema type annotations.
    ///
    /// The default implementation returns fields unchanged.
    ///
    /// # Arguments
    ///
    /// * `fields` - The normalized document fields
    /// * `schema` - The Quill schema (JSON Schema) for field type information
    ///
    /// # Returns
    ///
    /// Transformed fields ready for JSON serialization
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark_core::{QuillValue, Backend};
    /// # use std::collections::HashMap;
    /// # struct MyBackend;
    /// # impl MyBackend {
    /// fn transform_fields(
    ///     &self,
    ///     fields: &HashMap<String, QuillValue>,
    ///     schema: &QuillValue,
    /// ) -> HashMap<String, QuillValue> {
    ///     // Transform markdown fields to backend-specific format
    ///     fields.clone()
    /// }
    /// # }
    /// ```
    fn transform_fields(
        &self,
        fields: &HashMap<String, QuillValue>,
        _schema: &QuillValue,
    ) -> HashMap<String, QuillValue> {
        // Default: return fields unchanged
        fields.clone()
    }
}
