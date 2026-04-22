//! # Orchestration
//!
//! Orchestrates the Quillmark engine and the renderable [`Quill`] type.
//!
//! ## Usage
//!
//! 1. Create an engine with [`Quillmark::new`]
//! 2. Load a quill with [`Quillmark::quill`] or [`Quillmark::quill_from_path`]
//! 3. Render documents directly via [`Quill::render`] or [`Quill::open`]

mod engine;
mod quill;

pub use engine::Quillmark;
pub use quill::Quill;

/// Ergonomic reference to a [`Quill`] object.
pub enum QuillRef<'a> {
    /// Reference to a borrowed Quill object
    Object(&'a Quill),
}

impl<'a> From<&'a Quill> for QuillRef<'a> {
    fn from(quill: &'a Quill) -> Self {
        QuillRef::Object(quill)
    }
}
