//! # Quillmark
//!
//! Quillmark is a schema-driven document engine that turns Markdown
//! with card-yaml metadata blocks into a fully typeset document (PDF, SVG, PNG).
//!
//! Markdown enters through the bound door: `Quill::parse` conforms the
//! document against the quill that will render it.
//!
//! ```no_run
//! use quillmark::{quill_from_path, OutputFormat, Quillmark, RenderOptions};
//!
//! let quill = quill_from_path("path/to/quill").unwrap();
//! let engine = Quillmark::new();
//!
//! let doc = quill.parse("~~~\n$quill: my_quill\n$kind: main\ntitle: Hello\n~~~\n\n# Hello World").unwrap().document;
//! let result = engine.render(&quill, &doc, &RenderOptions::default().with_output_format(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! Or no Markdown at all: a blank canvas and the schema-bound writer.
//!
//! ```no_run
//! use quillmark::{quill_from_path, Document};
//!
//! let quill = quill_from_path("path/to/quill").unwrap();
//! let mut doc = Document::new("my_quill".parse().unwrap());
//!
//! let mut writer = quill.writer(&mut doc);
//! writer.set("title", "Hello").unwrap();
//! ```

// Every flow the docs name (quill construction, authoring, render, reading the
// result back) is spellable through this facade alone, with no direct
// `quillmark-core` dependency; `tests/facade_surface.rs` is the gate. `Quill`
// is the single quill type (portable, declarative data); construct it from an
// in-memory tree with `Quill::from_tree` (taking `FileTreeNode` and
// `QuillIgnore` from here) or from disk with the `quill_from_path` helper
// below. `QuillConfig` and the schema types come along, since a caller holding
// a `Quill` reads its schema through them.
pub use quillmark_core::{
    Artifact, Backend, BoundParseError, Card, CardSchema, ChangeSet, Content, Delta, Diagnostic,
    Document, EditError, FieldSchema, FieldType, FileTreeNode, LiveSession, Location, OutputFormat,
    ParseError, Parsed, Quill, QuillConfig, QuillIgnore, QuillReference, QuillValue, RenderError,
    RenderOptions, RenderResult, Severity, TypedWriter, ValidationError,
};

mod load;
pub mod orchestration;

pub use load::{quill_from_path, quill_from_path_with_warnings, tree_from_path};
pub use orchestration::Quillmark;
