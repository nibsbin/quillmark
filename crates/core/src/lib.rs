//! # Quillmark Core Overview
//!
//! Core types and functionality for the Quillmark template-first Markdown rendering system.
//!
//! ## Features
//!
//! This crate provides the foundational types and traits for Quillmark:
//!
//! - **Parsing**: YAML frontmatter extraction with Extended YAML Metadata Standard support
//! - **Templating**: MiniJinja-based template composition with stable filter API
//! - **Template model**: [`Quill`] type for managing template bundles with in-memory file system
//! - **Backend trait**: Extensible interface for implementing output format backends
//! - **Error handling**: Structured diagnostics with source location tracking
//! - **Utilities**: TOML⇄YAML conversion helpers
//!
//! ## Quick Start
//!
//! ```no_run
//! use quillmark_core::{ParsedDocument, Quill};
//!
//! // Parse markdown with frontmatter
//! let markdown = "---\ntitle: Example\n---\n\n# Content";
//! let doc = ParsedDocument::from_markdown(markdown);
//!
//! // Load a quill template
//! let quill = Quill::from_path("path/to/quill").unwrap();
//! ```
//!
//! ## Architecture
//!
//! The crate is organized into modules:
//!
//! - [`parse`]: Markdown parsing with YAML frontmatter support
//! - [`templating`]: Template composition using MiniJinja
//! - [`backend`]: Backend trait for output format implementations
//! - [`error`]: Structured error handling and diagnostics
//! - [`types`]: Core rendering types (OutputFormat, Artifact, RenderOptions)
//! - [`quill`]: Quill template bundle and related types
//!
//! ## Further Reading
//!
//! - [PARSE.md](https://github.com/nibsbin/quillmark/blob/main/designs/PARSE.md) - Detailed parsing documentation
//! - [Examples](https://github.com/nibsbin/quillmark/tree/main/examples) - Working examples

pub mod parse;
pub use parse::{ParsedDocument, BODY_FIELD};

pub mod templating;
pub use templating::{Plate, TemplateError};

pub mod backend;
pub use backend::Backend;

pub mod error;
pub use error::{
    Diagnostic, Location, ParseError, RenderError, RenderResult, SerializableDiagnostic, Severity,
};

pub mod types;
pub use types::{Artifact, OutputFormat, RenderOptions};

pub mod quill;
pub use quill::{FileTreeNode, Quill, QuillIgnore};

pub mod value;
pub use value::QuillValue;

pub mod schema;

pub mod guillemet;
pub use guillemet::{
    preprocess_fields_guillemets, preprocess_guillemets, preprocess_markdown_guillemets,
    strip_chevrons,
};

pub mod normalize;
pub use normalize::{
    normalize_document, normalize_fields, normalize_markdown, strip_bidi_formatting,
    NormalizationError,
};
