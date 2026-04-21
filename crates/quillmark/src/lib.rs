//! # Quillmark
//!
//! Quillmark is a flexible, format-first Markdown rendering system that converts Markdown
//! with YAML frontmatter into various output artifacts (PDF, SVG, TXT, etc.).
//!
//! ## Quick Start
//!
//! ```no_run
//! use quillmark::{Quillmark, OutputFormat, Document};
//!
//! let engine = Quillmark::new();
//! let quill = engine.quill_from_path("path/to/quill").unwrap();
//! let workflow = engine.workflow(&quill).unwrap();
//!
//! let parsed = Document::from_markdown("---\nQUILL: my_quill\ntitle: Hello\n---\n# Hello World").unwrap();
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```

// Re-export all core types for convenience
pub use quillmark_core::{
    Artifact, Backend, Card, Diagnostic, Document, Location, OutputFormat, ParseError, ParseOutput,
    Quill, RenderError, RenderOptions, RenderResult, RenderSession, SerializableDiagnostic,
    Severity,
};

// Declare orchestration module
pub mod orchestration;

// Re-export types from orchestration module
pub use orchestration::{QuillRef, Quillmark, Workflow};
