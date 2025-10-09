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
//! use quillmark_core::{decompose, Quill};
//!
//! // Parse markdown with frontmatter
//! let markdown = "---\ntitle: Example\n---\n\n# Content";
//! let doc = decompose(markdown).unwrap();
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
pub use parse::{decompose, ParsedDocument, BODY_FIELD};

pub mod templating;
pub use templating::{Glue, TemplateError};

pub mod backend;
pub use backend::Backend;

pub mod error;
pub use error::{Diagnostic, Location, ParseError, RenderError, RenderResult, Severity};

pub mod types;
pub use types::{Artifact, OutputFormat, RenderOptions};

pub mod quill;
pub use quill::{FieldSchema, FileTreeNode, Quill, QuillIgnore};

pub mod value;
pub use value::QuillValue;
