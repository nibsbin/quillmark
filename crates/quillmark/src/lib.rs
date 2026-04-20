//! # Quillmark
//!
//! Quillmark is a flexible, format-first Markdown rendering system that converts Markdown
//! with YAML frontmatter into various output artifacts (PDF, SVG, TXT, etc.).
//!
//! ## Quick Start
//!
//! ```no_run
//! use quillmark::{Quillmark, OutputFormat, ParsedDocument};
//!
//! let engine = Quillmark::new();
//! let quill = engine.quill_from_path("path/to/quill").unwrap();
//! let workflow = engine.workflow(&quill).unwrap();
//!
//! let parsed = ParsedDocument::from_markdown("---\ntitle: Hello\n---\n# Hello World").unwrap();
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```

// Re-export all core types for convenience
pub use quillmark_core::{
    Artifact, Backend, Diagnostic, Location, OutputFormat, ParseError, ParsedDocument, Quill,
    RenderError, RenderOptions, RenderResult, SerializableDiagnostic, Severity,
    BODY_FIELD,
};

// Declare orchestration module
pub mod orchestration;

// Re-export types from orchestration module
pub use orchestration::{QuillRef, Quillmark, Workflow};
