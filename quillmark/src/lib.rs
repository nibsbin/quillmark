//! Flexible template-first Markdown rendering system. See [module docs](self) for examples and usage patterns.

#[doc = include_str!("../docs/lib.md")]

// Re-export all core types for backward compatibility
pub use quillmark_core::{
    decompose, Artifact, Backend, Diagnostic, Glue, Location, OutputFormat, ParsedDocument, Quill,
    RenderError, RenderResult, Severity, TemplateError, BODY_FIELD,
};

// Declare modules
pub mod workflow;
pub mod quillmark;

// Re-export types from modules for backward compatibility
pub use workflow::Workflow;
pub use quillmark::{Quillmark, QuillRef};
