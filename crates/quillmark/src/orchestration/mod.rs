//! The render dispatcher over the portable
//! [`Quill`](quillmark_core::quill::Quill) type, which lives in core and needs
//! no `Quillmark` to load.

mod engine;

pub use engine::Quillmark;
