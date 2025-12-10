//! # Orchestration
//!
//! Orchestrates the Quillmark engine and its workflows.
//!
//! ---
//!
//! # Quillmark Engine
//!
//! High-level engine for orchestrating backends and quills.
//!
//! [`Quillmark`] manages the registration of backends and quills, and provides
//! a convenient way to create workflows. Backends are automatically registered
//! based on enabled crate features.
//!
//! ## Backend Auto-Registration
//!
//! When a [`Quillmark`] engine is created with [`Quillmark::new`], it automatically
//! registers all backends based on enabled features:
//!
//! - **typst** (default) - Typst backend for PDF/SVG rendering
//!
//! ## Workflow (Engine Level)
//!
//! 1. Create an engine with [`Quillmark::new`]
//! 2. Register quills with [`Quillmark::register_quill()`]
//! 3. Load workflows with [`Quillmark::workflow()`]
//! 4. Render documents using the workflow
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```no_run
//! use quillmark::{Quillmark, Quill, OutputFormat, ParsedDocument};
//!
//! // Step 1: Create engine with auto-registered backends
//! let mut engine = Quillmark::new();
//!
//! // Step 2: Create and register quills
//! let quill = Quill::from_path("path/to/quill").unwrap();
//! engine.register_quill(quill);
//!
//! // Step 3: Parse markdown
//! let markdown = "# Hello";
//! let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//!
//! // Step 4: Load workflow and render
//! let workflow = engine.workflow("my-quill").unwrap();
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! ### Loading by Reference
//!
//! ```no_run
//! # use quillmark::{Quillmark, Quill, ParsedDocument};
//! # let mut engine = Quillmark::new();
//! let quill = Quill::from_path("path/to/quill").unwrap();
//! engine.register_quill(quill.clone());
//!
//! // Load by name
//! let workflow1 = engine.workflow("my-quill").unwrap();
//!
//! // Load by object (doesn't need to be registered)
//! let workflow2 = engine.workflow(&quill).unwrap();
//!
//! // Load from parsed document
//! let parsed = ParsedDocument::from_markdown("---\nQUILL: my-quill\n---\n# Hello").unwrap();
//! let workflow3 = engine.workflow(&parsed).unwrap();
//! ```
//!
//! ### Inspecting Engine State
//!
//! ```no_run
//! # use quillmark::Quillmark;
//! # let engine = Quillmark::new();
//! println!("Available backends: {:?}", engine.registered_backends());
//! println!("Registered quills: {:?}", engine.registered_quills());
//! ```
//!
//! ---
//!
//! # Workflow
//!
//! Sealed workflow for rendering Markdown documents.
//!
//! [`Workflow`] encapsulates the complete rendering pipeline from Markdown to final artifacts.
//! It manages the backend, quill template, and dynamic assets, providing methods for
//! rendering at different stages of the pipeline.
//!
//! ## Rendering Pipeline
//!
//! The workflow supports rendering at three levels:
//!
//! 1. **Full render** ([`Workflow::render()`]) - Compose with template → Compile to artifacts (parsing done separately)
//! 2. **Content render** ([`Workflow::render_plate()`]) - Skip parsing, render pre-composed content
//! 3. **Plate only** ([`Workflow::process_plate()`]) - Compose from parsed document, return template output
//!
//! ## Examples
//!
//! ### Basic Rendering
//!
//! ```no_run
//! # use quillmark::{Quillmark, OutputFormat, ParsedDocument};
//! # let mut engine = Quillmark::new();
//! # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
//! # engine.register_quill(quill);
//! let workflow = engine.workflow("my-quill").unwrap();
//!
//! let markdown = r#"---
//! title: "My Document"
//! author: "Alice"
//! ---
//!
//! # Introduction
//!
//! This is my document.
//! "#;
//!
//! let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! ### Dynamic Assets
//!
//! ```no_run
//! # use quillmark::{Quillmark, OutputFormat, ParsedDocument};
//! # let mut engine = Quillmark::new();
//! # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
//! # engine.register_quill(quill);
//! # let markdown = "# Report";
//! # let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//! let mut workflow = engine.workflow("my-quill").unwrap();
//! workflow.add_asset("logo.png", vec![/* PNG bytes */]).unwrap();
//! workflow.add_asset("chart.svg", vec![/* SVG bytes */]).unwrap();
//!
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! ### Dynamic Fonts
//!
//! ```no_run
//! # use quillmark::{Quillmark, OutputFormat, ParsedDocument};
//! # let mut engine = Quillmark::new();
//! # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
//! # engine.register_quill(quill);
//! # let markdown = "# Report";
//! # let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//! let mut workflow = engine.workflow("my-quill").unwrap();
//! workflow.add_font("custom-font.ttf", vec![/* TTF bytes */]).unwrap();
//! workflow.add_font("another-font.otf", vec![/* OTF bytes */]).unwrap();
//!
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! ### Inspecting Workflow Properties
//!
//! ```no_run
//! # use quillmark::Quillmark;
//! # let mut engine = Quillmark::new();
//! # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
//! # engine.register_quill(quill);
//! let workflow = engine.workflow("my-quill").unwrap();
//!
//! println!("Backend: {}", workflow.backend_id());
//! println!("Quill: {}", workflow.quill_name());
//! println!("Formats: {:?}", workflow.supported_formats());
//! ```

mod engine;
mod workflow;

pub use engine::Quillmark;
pub use workflow::Workflow;

use quillmark_core::{ParsedDocument, Quill};

/// Ergonomic reference to a Quill by name or object.
pub enum QuillRef<'a> {
    /// Reference to a quill by its registered name
    Name(&'a str),
    /// Reference to a borrowed Quill object
    Object(&'a Quill),
    /// Reference to a parsed document (extracts quill tag)
    Parsed(&'a ParsedDocument),
}

impl<'a> From<&'a Quill> for QuillRef<'a> {
    fn from(quill: &'a Quill) -> Self {
        QuillRef::Object(quill)
    }
}

impl<'a> From<&'a str> for QuillRef<'a> {
    fn from(name: &'a str) -> Self {
        QuillRef::Name(name)
    }
}

impl<'a> From<&'a String> for QuillRef<'a> {
    fn from(name: &'a String) -> Self {
        QuillRef::Name(name.as_str())
    }
}

impl<'a> From<&'a std::borrow::Cow<'a, str>> for QuillRef<'a> {
    fn from(name: &'a std::borrow::Cow<'a, str>) -> Self {
        QuillRef::Name(name.as_ref())
    }
}

impl<'a> From<&'a ParsedDocument> for QuillRef<'a> {
    fn from(parsed: &'a ParsedDocument) -> Self {
        QuillRef::Parsed(parsed)
    }
}
