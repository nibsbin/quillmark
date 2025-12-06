//! # Orchestration
//!
//! Orchestrates the Quillmark engine and its workflows.
//!
//! ---
//!
//! # Quillmark Engine
//!
//! High-level engine for orchestrating backends and plates.
//!
//! [`Quillmark`] manages the registration of backends and plates, and provides
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
//! 2. Register plates with [`Quillmark::register_plate()`]
//! 3. Load workflows with [`Quillmark::workflow()`]
//! 4. Render documents using the workflow
//!
//! ## Examples
//!
//! ### Basic Usage
//!
//! ```no_run
//! use quillmark::{Quillmark, Plate, OutputFormat, ParsedDocument};
//!
//! // Step 1: Create engine with auto-registered backends
//! let mut engine = Quillmark::new();
//!
//! // Step 2: Create and register plates
//! let plate = Plate::from_path("path/to/plate").unwrap();
//! engine.register_plate(plate);
//!
//! // Step 3: Parse markdown
//! let markdown = "# Hello";
//! let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//!
//! // Step 4: Load workflow and render
//! let workflow = engine.workflow("my-plate").unwrap();
//! let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
//! ```
//!
//! ### Loading by Reference
//!
//! ```no_run
//! # use quillmark::{Quillmark, Plate, ParsedDocument};
//! # let mut engine = Quillmark::new();
//! let plate = Plate::from_path("path/to/plate").unwrap();
//! engine.register_plate(plate.clone());
//!
//! // Load by name
//! let workflow1 = engine.workflow("my-plate").unwrap();
//!
//! // Load by object (doesn't need to be registered)
//! let workflow2 = engine.workflow(&plate).unwrap();
//!
//! // Load from parsed document
//! let parsed = ParsedDocument::from_markdown("---\nPLATE: my-plate\n---\n# Hello").unwrap();
//! let workflow3 = engine.workflow(&parsed).unwrap();
//! ```
//!
//! ### Inspecting Engine State
//!
//! ```no_run
//! # use quillmark::Quillmark;
//! # let engine = Quillmark::new();
//! println!("Available backends: {:?}", engine.registered_backends());
//! println!("Registered plates: {:?}", engine.registered_plates());
//! ```
//!
//! ---
//!
//! # Workflow
//!
//! Sealed workflow for rendering Markdown documents.
//!
//! [`Workflow`] encapsulates the complete rendering pipeline from Markdown to final artifacts.
//! It manages the backend, plate template, and dynamic assets, providing methods for
//! rendering at different stages of the pipeline.
//!
//! ## Rendering Pipeline
//!
//! The workflow supports rendering at three levels:
//!
//! 1. **Full render** ([`Workflow::render()`]) - Compose with template → Compile to artifacts (parsing done separately)
//! 2. **Content render** ([`Workflow::render_processed()`]) - Skip parsing, render pre-composed content
//! 3. **Glue only** ([`Workflow::process_glue()`]) - Compose from parsed document, return template output
//!
//! ## Examples
//!
//! ### Basic Rendering
//!
//! ```no_run
//! # use quillmark::{Quillmark, OutputFormat, ParsedDocument};
//! # let mut engine = Quillmark::new();
//! # let plate = quillmark::Plate::from_path("path/to/plate").unwrap();
//! # engine.register_plate(plate);
//! let workflow = engine.workflow("my-plate").unwrap();
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
//! # let plate = quillmark::Plate::from_path("path/to/plate").unwrap();
//! # engine.register_plate(plate);
//! # let markdown = "# Report";
//! # let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//! let mut workflow = engine.workflow("my-plate").unwrap();
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
//! # let plate = quillmark::Plate::from_path("path/to/plate").unwrap();
//! # engine.register_plate(plate);
//! # let markdown = "# Report";
//! # let parsed = ParsedDocument::from_markdown(markdown).unwrap();
//! let mut workflow = engine.workflow("my-plate").unwrap();
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
//! # let plate = quillmark::Plate::from_path("path/to/plate").unwrap();
//! # engine.register_plate(plate);
//! let workflow = engine.workflow("my-plate").unwrap();
//!
//! println!("Backend: {}", workflow.backend_id());
//! println!("Plate: {}", workflow.plate_name());
//! println!("Formats: {:?}", workflow.supported_formats());
//! ```

mod engine;
mod workflow;

pub use engine::Quillmark;
pub use workflow::Workflow;

use quillmark_core::{ParsedDocument, Plate};

/// Ergonomic reference to a Plate by name or object.
pub enum PlateRef<'a> {
    /// Reference to a plate by its registered name
    Name(&'a str),
    /// Reference to a borrowed Plate object
    Object(&'a Plate),
    /// Reference to a parsed document (extracts plate tag)
    Parsed(&'a ParsedDocument),
}

impl<'a> From<&'a Plate> for PlateRef<'a> {
    fn from(plate: &'a Plate) -> Self {
        PlateRef::Object(plate)
    }
}

impl<'a> From<&'a str> for PlateRef<'a> {
    fn from(name: &'a str) -> Self {
        PlateRef::Name(name)
    }
}

impl<'a> From<&'a String> for PlateRef<'a> {
    fn from(name: &'a String) -> Self {
        PlateRef::Name(name.as_str())
    }
}

impl<'a> From<&'a std::borrow::Cow<'a, str>> for PlateRef<'a> {
    fn from(name: &'a std::borrow::Cow<'a, str>) -> Self {
        PlateRef::Name(name.as_ref())
    }
}

impl<'a> From<&'a ParsedDocument> for PlateRef<'a> {
    fn from(parsed: &'a ParsedDocument) -> Self {
        PlateRef::Parsed(parsed)
    }
}
