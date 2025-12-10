//! # Quillmark
//!
//! Quillmark is a flexible, template-first Markdown rendering system that converts Markdown
//! with YAML frontmatter into various output artifacts (PDF, SVG, TXT, etc.).
//!
//! ## Overview
//!
//! Quillmark uses a **sealed engine API** that orchestrates the rendering workflow through
//! three main stages:
//!
//! 1. **Parsing** - YAML frontmatter and body extraction from Markdown
//! 2. **Templating** - MiniJinja-based composition with backend-registered filters
//! 3. **Backend Processing** - Compilation of composed content to final artifacts
//!
//! ## Core Components
//!
//! - [`Quillmark`] - High-level engine for managing backends and quills
//! - [`Workflow`] - Sealed rendering API for executing the render pipeline
//! - [`QuillRef`] - Ergonomic references to quills (by name or object)
//! - [`Quill`] - Template bundle containing plate templates and assets
//!
//! ## Quick Start
//!
//! ```no_run
//! use quillmark::{Quillmark, Quill, OutputFormat, ParsedDocument};
//!
//! // Create engine with auto-registered backends
//! let mut engine = Quillmark::new();
//!
//! // Load and register a quill template
//! let quill = Quill::from_path("path/to/quill").unwrap();
//! engine.register_quill(quill);
//!
//! // Parse markdown
//! let markdown = "---\ntitle: Hello\n---\n# Hello World";
//! let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//!
//! // Create a workflow and render
//! let workflow = engine.workflow("my-quill").unwrap();
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//!
//! // Access the rendered artifacts
//! for artifact in result.artifacts {
//!     println!("Generated {} bytes of {:?}", artifact.bytes.len(), artifact.output_format);
//! }
//! ```
//!
//! ## Dynamic Assets
//!
//! Workflows support adding runtime assets:
//!
//! ```no_run
//! # use quillmark::{Quillmark, Quill, OutputFormat, ParsedDocument};
//! # let mut engine = Quillmark::new();
//! # let quill = Quill::from_path("path/to/quill").unwrap();
//! # engine.register_quill(quill);
//! # let markdown = "# Report";
//! # let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//! let mut workflow = engine.workflow("my-quill").unwrap();
//! workflow.add_asset("chart.png", vec![/* image bytes */]).unwrap();
//! workflow.add_asset("data.csv", vec![/* csv bytes */]).unwrap();
//!
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! ## Features
//!
//! - **typst** (enabled by default) - Typst backend for PDF/SVG rendering
//!
//! ## Custom Backends
//!
//! Third-party backends can be registered with a Quillmark engine:
//!
//! ```no_run
//! use quillmark::{Quillmark, Backend};
//! # use quillmark_core::{OutputFormat, Plate, Quill, RenderOptions, Artifact, RenderError, RenderResult};
//! # struct MyCustomBackend;
//! # impl Backend for MyCustomBackend {
//! #     fn id(&self) -> &'static str { "custom" }
//! #     fn supported_formats(&self) -> &'static [OutputFormat] { &[OutputFormat::Txt] }
//! #     fn plate_extension_types(&self) -> &'static [&'static str] { &[".txt"] }
//! #     fn allow_auto_plate(&self) -> bool { true }
//! #     fn register_filters(&self, _plate: &mut Plate) {}
//! #     fn compile(&self, content: &str, _quill: &Quill, _opts: &RenderOptions) -> Result<RenderResult, RenderError> {
//! #         let artifacts = vec![Artifact { bytes: content.as_bytes().to_vec(), output_format: OutputFormat::Txt }];
//! #         Ok(RenderResult::new(artifacts, OutputFormat::Txt))
//! #     }
//! # }
//!
//! let mut engine = Quillmark::new();
//!
//! // Register a custom backend
//! let custom_backend = Box::new(MyCustomBackend);
//! engine.register_backend(custom_backend);
//! ```
//!
//! ## Re-exported Types
//!
//! This crate re-exports commonly used types from `quillmark-core` for convenience.

// Re-export all core types for convenience
pub use quillmark_core::{
    Artifact, Backend, Diagnostic, Location, OutputFormat, ParseError, ParsedDocument, Plate,
    Quill, RenderError, RenderResult, SerializableDiagnostic, Severity, TemplateError, BODY_FIELD,
};

// Declare orchestration module
pub mod orchestration;

// Re-export types from orchestration module
pub use orchestration::{QuillRef, Quillmark, Workflow};
