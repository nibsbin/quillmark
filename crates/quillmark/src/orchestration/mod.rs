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
