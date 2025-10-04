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
//! - [`Quill`] - Template bundle containing glue templates and assets
//!
//! ## Quick Start
//!
//! ```no_run
//! use quillmark::{Quillmark, Quill, OutputFormat};
//!
//! // Create engine with auto-registered backends
//! let mut engine = Quillmark::new();
//!
//! // Load and register a quill template
//! let quill = Quill::from_path("path/to/quill").unwrap();
//! engine.register_quill(quill);
//!
//! // Create a workflow and render markdown
//! let workflow = engine.load("my-quill").unwrap();
//! let result = workflow.render(
//!     "---\ntitle: Hello\n---\n# Hello World",
//!     Some(OutputFormat::Pdf)
//! ).unwrap();
//!
//! // Access the rendered artifacts
//! for artifact in result.artifacts {
//!     println!("Generated {} bytes of {:?}", artifact.bytes.len(), artifact.output_format);
//! }
//! ```
//!
//! ## Dynamic Assets
//!
//! Workflows support adding runtime assets through a builder pattern:
//!
//! ```no_run
//! # use quillmark::{Quillmark, Quill, OutputFormat};
//! # let mut engine = Quillmark::new();
//! # let quill = Quill::from_path("path/to/quill").unwrap();
//! # engine.register_quill(quill);
//! let workflow = engine.load("my-quill").unwrap()
//!     .with_asset("chart.png", vec![/* image bytes */]).unwrap()
//!     .with_asset("data.csv", vec![/* csv bytes */]).unwrap();
//!
//! let result = workflow.render("# Report", Some(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! ## Features
//!
//! - **typst** (enabled by default) - Typst backend for PDF/SVG rendering
//!
//! ## Re-exported Types
//!
//! This crate re-exports commonly used types from `quillmark-core` for convenience.

// Re-export all core types for backward compatibility
pub use quillmark_core::{
    decompose, Artifact, Backend, Diagnostic, Glue, Location, OutputFormat, ParsedDocument, Quill,
    RenderError, RenderResult, Severity, TemplateError, BODY_FIELD,
};

// Declare orchestration module
pub mod orchestration;

// Re-export types from orchestration module for backward compatibility
pub use orchestration::{QuillRef, Quillmark, Workflow};
