//! # Orchestration
//!
//! Orchestrates the Quillmark engine and its workflows.
//!
//! ## Workflow
//!
//! 1. Create an engine with [`Quillmark::new`]
//! 2. Load a quill with [`Quillmark::quill`] or [`Quillmark::quill_from_path`]
//! 3. Create a workflow with [`Quillmark::workflow`]
//! 4. Render documents using the workflow

mod engine;
mod workflow;

pub use engine::Quillmark;
pub use workflow::Workflow;

use quillmark_core::Quill;

/// Ergonomic reference to a Quill object.
pub enum QuillRef<'a> {
    /// Reference to a borrowed Quill object
    Object(&'a Quill),
}

impl<'a> From<&'a Quill> for QuillRef<'a> {
    fn from(quill: &'a Quill) -> Self {
        QuillRef::Object(quill)
    }
}
