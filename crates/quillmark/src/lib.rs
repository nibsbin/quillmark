//! # Quillmark
//!
//! Quillmark is a schema-driven document engine that turns Markdown
//! with card-yaml metadata blocks into a fully typeset document (PDF, SVG, PNG).
//!
//! ```no_run
//! use quillmark::{quill_from_path, Document, OutputFormat, Quillmark, RenderOptions};
//!
//! let quill = quill_from_path("path/to/quill").unwrap();
//! let engine = Quillmark::new();
//!
//! let parsed = Document::parse("~~~\n$quill: my_quill\n$kind: main\ntitle: Hello\n~~~\n\n# Hello World").unwrap().document;
//! let result = engine.render(&quill, &parsed, &RenderOptions {
//!     output_format: Some(OutputFormat::Pdf),
//!     ..Default::default()
//! }).unwrap();
//! ```

// Re-export core types for convenience. `Quill` is the single quill type
// (portable, declarative data); construct it from an in-memory tree with
// `Quill::from_tree` — taking `FileTreeNode` and `QuillIgnore` from here — or
// from disk with the `quill_from_path` helper below. Neither path needs a
// direct `quillmark-core` dependency. `QuillConfig` and the schema types come
// along, since a caller holding a `Quill` reads its schema through them.
pub use quillmark_core::{
    Artifact, Backend, Card, CardSchema, ChangeSet, Delta, Diagnostic, Document, FieldSchema,
    FieldType, FileTreeNode, LiveSession, Location, OutputFormat, ParseError, Parsed, Quill,
    QuillConfig, QuillIgnore, RenderError, RenderOptions, RenderResult, ValidationError,
    Content, Severity,
};

mod load;
pub mod orchestration;

pub use load::{quill_from_path, quill_from_path_with_warnings, tree_from_path};
pub use orchestration::Quillmark;
