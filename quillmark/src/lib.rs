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

use quillmark_core::RenderOptions;
use std::collections::HashMap;

/// Reference to a Quill, either by name or by borrowed object.
///
/// `QuillRef` provides an ergonomic way to reference quills when loading workflows.
/// It automatically converts from common string types and quill references.
///
/// # Examples
///
/// ```no_run
/// # use quillmark::{Quillmark, Quill, QuillRef};
/// # let mut engine = Quillmark::new();
/// # let quill = Quill::from_path("path/to/quill").unwrap();
/// # engine.register_quill(quill.clone());
/// // All of these work:
/// let workflow = engine.load("my-quill").unwrap();           // &str
/// let workflow = engine.load(&String::from("my-quill")).unwrap();  // &String
/// let workflow = engine.load(&quill).unwrap();               // &Quill
/// ```
pub enum QuillRef<'a> {
    /// Reference to a quill by its registered name
    Name(&'a str),
    /// Reference to a borrowed Quill object
    Object(&'a Quill),
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

/// Sealed workflow for rendering Markdown documents.
///
/// `Workflow` encapsulates the complete rendering pipeline from Markdown to final artifacts.
/// It manages the backend, quill template, and dynamic assets, providing methods for
/// rendering at different stages of the pipeline.
///
/// # Rendering Pipeline
///
/// The workflow supports rendering at three levels:
///
/// 1. **Full render** ([`render`](Self::render)) - Parse Markdown → Compose with template → Compile to artifacts
/// 2. **Content render** ([`render_content`](Self::render_content)) - Skip parsing, render pre-composed content
/// 3. **Glue only** ([`process_glue`](Self::process_glue)) - Parse and compose, return template output
///
/// # Examples
///
/// ## Basic Rendering
///
/// ```no_run
/// # use quillmark::{Quillmark, OutputFormat};
/// # let mut engine = Quillmark::new();
/// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
/// # engine.register_quill(quill);
/// let workflow = engine.load("my-quill").unwrap();
///
/// let markdown = r#"---
/// title: "My Document"
/// author: "Alice"
/// ---
///
/// # Introduction
///
/// This is my document.
/// "#;
///
/// let result = workflow.render(markdown, Some(OutputFormat::Pdf)).unwrap();
/// ```
///
/// ## Dynamic Assets (Builder Pattern)
///
/// ```no_run
/// # use quillmark::{Quillmark, OutputFormat};
/// # let mut engine = Quillmark::new();
/// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
/// # engine.register_quill(quill);
/// let workflow = engine.load("my-quill").unwrap()
///     .with_asset("logo.png", vec![/* PNG bytes */]).unwrap()
///     .with_asset("chart.svg", vec![/* SVG bytes */]).unwrap();
///
/// let result = workflow.render("# Report", Some(OutputFormat::Pdf)).unwrap();
/// ```
///
/// ## Inspecting Workflow Properties
///
/// ```no_run
/// # use quillmark::Quillmark;
/// # let mut engine = Quillmark::new();
/// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
/// # engine.register_quill(quill);
/// let workflow = engine.load("my-quill").unwrap();
///
/// println!("Backend: {}", workflow.backend_id());
/// println!("Quill: {}", workflow.quill_name());
/// println!("Formats: {:?}", workflow.supported_formats());
/// ```
pub struct Workflow {
    backend: Box<dyn Backend>,
    quill: Quill,
    dynamic_assets: HashMap<String, Vec<u8>>,
}

impl Workflow {
    /// Create a new Workflow with the specified backend and quill template.
    ///
    /// This is typically called internally by [`Quillmark::load`]. Most users should use
    /// the engine to create workflows rather than calling this directly.
    ///
    /// # Arguments
    ///
    /// * `backend` - The backend implementation to use for compilation
    /// * `quill` - The quill template bundle to use for rendering
    ///
    /// # Returns
    ///
    /// Returns `Ok(Workflow)` if the workflow was successfully created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::{Workflow, Quill};
    /// # #[cfg(feature = "typst")]
    /// # {
    /// let backend = Box::new(quillmark_typst::TypstBackend::default());
    /// let quill = Quill::from_path("path/to/quill").unwrap();
    /// let workflow = Workflow::new(backend, quill).unwrap();
    /// # }
    /// ```
    pub fn new(backend: Box<dyn Backend>, quill: Quill) -> Result<Self, RenderError> {
        // Since Quill::from_path() now automatically validates, we don't need to validate again
        Ok(Self {
            backend,
            quill,
            dynamic_assets: HashMap::new(),
        })
    }

    /// Render Markdown with YAML frontmatter to output artifacts.
    ///
    /// This is the primary rendering method. It performs the complete pipeline:
    /// 1. Parse the Markdown and extract YAML frontmatter
    /// 2. Compose the content with the glue template using MiniJinja
    /// 3. Compile the composed content using the backend
    ///
    /// # Arguments
    ///
    /// * `markdown` - Markdown content with optional YAML frontmatter
    /// * `format` - Optional output format (e.g., PDF, SVG). If `None`, uses the backend's first supported format
    ///
    /// # Returns
    ///
    /// Returns a [`RenderResult`] containing the generated artifacts and any warnings.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if:
    /// - The YAML frontmatter is invalid
    /// - Template composition fails
    /// - Backend compilation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, OutputFormat};
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap();
    ///
    /// // With frontmatter
    /// let result = workflow.render(
    ///     "---\ntitle: Hello\n---\n# Content",
    ///     Some(OutputFormat::Pdf)
    /// ).unwrap();
    ///
    /// // Without frontmatter
    /// let result = workflow.render("# Simple Content", None).unwrap();
    /// ```
    pub fn render(
        &self,
        markdown: &str,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        let glue_output = self.process_glue(markdown)?;

        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();

        // Pass prepared quill to backend
        self.render_content_with_quill(&glue_output, format, &prepared_quill)
    }

    /// Render pre-processed glue content to output artifacts.
    ///
    /// This method skips the parsing and template composition steps, directly compiling
    /// the provided content using the backend. Useful when you have already processed
    /// the glue template or want to provide backend-specific markup directly.
    ///
    /// # Arguments
    ///
    /// * `content` - Pre-composed glue content (e.g., Typst markup)
    /// * `format` - Optional output format. If `None`, uses the backend's first supported format
    ///
    /// # Returns
    ///
    /// Returns a [`RenderResult`] containing the generated artifacts and any warnings.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if backend compilation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, OutputFormat};
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap();
    ///
    /// // For Typst backend, you might provide Typst markup directly
    /// let typst_content = "#heading[My Document]\n#par[Content here]";
    /// let result = workflow.render_content(typst_content, Some(OutputFormat::Pdf)).unwrap();
    /// ```
    pub fn render_content(
        &self,
        content: &str,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();
        self.render_content_with_quill(content, format, &prepared_quill)
    }

    /// Internal method to render content with a specific quill
    fn render_content_with_quill(
        &self,
        content: &str,
        format: Option<OutputFormat>,
        quill: &Quill,
    ) -> Result<RenderResult, RenderError> {
        // Compile using backend
        let format = if format.is_some() {
            format
        } else {
            // Default to first supported format if none specified
            let supported = self.backend.supported_formats();
            if !supported.is_empty() {
                println!("Defaulting to output format: {:?}", supported[0]);
                Some(supported[0])
            } else {
                None
            }
        };
        // Compile using backend
        let render_opts = RenderOptions {
            output_format: format,
        };

        let artifacts = self.backend.compile(content, quill, &render_opts)?;
        Ok(RenderResult::new(artifacts))
    }

    /// Process Markdown through the glue template without compilation.
    ///
    /// This method performs only the parsing and template composition steps, returning
    /// the composed glue output (e.g., Typst markup) without compiling it to a final format.
    /// Useful for debugging templates or obtaining intermediate representations.
    ///
    /// # Arguments
    ///
    /// * `markdown` - Markdown content with optional YAML frontmatter
    ///
    /// # Returns
    ///
    /// Returns the composed glue content as a `String`.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if:
    /// - The YAML frontmatter is invalid
    /// - Template composition fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap();
    ///
    /// let markdown = "---\ntitle: Test\n---\n# Hello";
    /// let glue_output = workflow.process_glue(markdown).unwrap();
    /// println!("Glue output:\n{}", glue_output);
    /// ```
    pub fn process_glue(&self, markdown: &str) -> Result<String, RenderError> {
        let parsed_doc = decompose(markdown).map_err(|e| RenderError::InvalidFrontmatter {
            diag: quillmark_core::error::Diagnostic::new(
                quillmark_core::error::Severity::Error,
                format!("Failed to parse markdown: {}", e),
            ),
            source: Some(anyhow::anyhow!(e)),
        })?;

        let mut glue = Glue::new(self.quill.glue_template.clone());
        self.backend.register_filters(&mut glue);
        let glue_output = glue
            .compose(parsed_doc.fields().clone())
            .map_err(|e| RenderError::from(e))?;
        Ok(glue_output)
    }

    /// Get the backend identifier.
    ///
    /// Returns the backend ID string (e.g., "typst", "latex").
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap();
    /// assert_eq!(workflow.backend_id(), "typst");
    /// ```
    pub fn backend_id(&self) -> &str {
        self.backend.id()
    }

    /// Get the supported output formats for this workflow's backend.
    ///
    /// Returns a slice of all output formats that the backend can produce.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, OutputFormat};
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap();
    /// let formats = workflow.supported_formats();
    /// println!("Supported formats: {:?}", formats);
    /// ```
    pub fn supported_formats(&self) -> &'static [OutputFormat] {
        self.backend.supported_formats()
    }

    /// Get the quill name.
    ///
    /// Returns the name of the quill template used by this workflow.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # let name = quill.name.clone();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap();
    /// println!("Using quill: {}", workflow.quill_name());
    /// ```
    pub fn quill_name(&self) -> &str {
        &self.quill.name
    }

    /// Add a dynamic asset to the workflow (builder pattern).
    ///
    /// Dynamic assets are injected into the quill's virtual file system at render time,
    /// making them available to templates via the `Asset` filter. Assets are stored under
    /// `assets/DYNAMIC_ASSET__<filename>` in the virtual file system.
    ///
    /// This method consumes `self` and returns a new `Workflow`, enabling builder-style chaining.
    ///
    /// # Arguments
    ///
    /// * `filename` - The filename to use (e.g., "chart.png", "data.csv")
    /// * `contents` - The file contents as bytes
    ///
    /// # Returns
    ///
    /// Returns `Ok(Workflow)` with the asset added, or an error if the filename already exists.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::DynamicAssetCollision`] if a dynamic asset with the same
    /// filename already exists in this workflow.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap()
    ///     .with_asset("logo.png", vec![0x89, 0x50, 0x4e, 0x47]).unwrap()
    ///     .with_asset("chart.svg", b"<svg>...</svg>".to_vec()).unwrap();
    /// ```
    pub fn with_asset(
        mut self,
        filename: impl Into<String>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<Self, RenderError> {
        let filename = filename.into();

        // Check for collision
        if self.dynamic_assets.contains_key(&filename) {
            return Err(RenderError::DynamicAssetCollision {
                filename: filename.clone(),
                message: format!(
                    "Dynamic asset '{}' already exists. Each asset filename must be unique.",
                    filename
                ),
            });
        }

        self.dynamic_assets.insert(filename, contents.into());
        Ok(self)
    }

    /// Add multiple dynamic assets at once (builder pattern).
    ///
    /// Convenience method for adding multiple assets. Each asset is validated individually,
    /// and the first collision will return an error.
    ///
    /// # Arguments
    ///
    /// * `assets` - An iterator of `(filename, contents)` tuples
    ///
    /// # Returns
    ///
    /// Returns `Ok(Workflow)` with all assets added, or an error on the first collision.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::DynamicAssetCollision`] if any filename already exists.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # use std::collections::HashMap;
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let assets = vec![
    ///     ("logo.png".to_string(), vec![1, 2, 3]),
    ///     ("data.csv".to_string(), vec![4, 5, 6]),
    /// ];
    ///
    /// let workflow = engine.load("my-quill").unwrap()
    ///     .with_assets(assets).unwrap();
    /// ```
    pub fn with_assets(
        mut self,
        assets: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<Self, RenderError> {
        for (filename, contents) in assets {
            self = self.with_asset(filename, contents)?;
        }
        Ok(self)
    }

    /// Clear all dynamic assets from the workflow (builder pattern).
    ///
    /// This method removes all previously added dynamic assets, allowing you to
    /// start fresh or conditionally reset the asset state in a builder chain.
    ///
    /// # Returns
    ///
    /// Returns the workflow with all dynamic assets removed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let mut engine = Quillmark::new();
    /// # let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill);
    /// let workflow = engine.load("my-quill").unwrap()
    ///     .with_asset("temp.png", vec![1, 2, 3]).unwrap()
    ///     .clear_assets()  // Remove all assets
    ///     .with_asset("final.png", vec![4, 5, 6]).unwrap();  // Add new ones
    /// ```
    pub fn clear_assets(mut self) -> Self {
        self.dynamic_assets.clear();
        self
    }

    /// Internal method to prepare a quill with dynamic assets
    fn prepare_quill_with_assets(&self) -> Quill {
        use std::path::PathBuf;

        let mut quill = self.quill.clone();

        // Add dynamic assets to the cloned quill's file system
        for (filename, contents) in &self.dynamic_assets {
            let prefixed_path = PathBuf::from(format!("assets/DYNAMIC_ASSET__{}", filename));
            let entry = quillmark_core::FileEntry {
                contents: contents.clone(),
                path: prefixed_path.clone(),
                is_dir: false,
            };
            quill.files.insert(prefixed_path, entry);
        }

        quill
    }
}

/// High-level engine for orchestrating backends and quills.
///
/// `Quillmark` manages the registration of backends and quills, and provides
/// a convenient way to create workflows. Backends are automatically registered
/// based on enabled crate features.
///
/// # Backend Auto-Registration
///
/// When a `Quillmark` engine is created with [`new`](Self::new), it automatically
/// registers all backends based on enabled features:
///
/// - **typst** (default) - Typst backend for PDF/SVG rendering
///
/// # Workflow
///
/// 1. Create an engine with [`Quillmark::new`]
/// 2. Register quills with [`register_quill`](Self::register_quill)
/// 3. Load workflows with [`load`](Self::load)
/// 4. Render documents using the workflow
///
/// # Examples
///
/// ## Basic Usage
///
/// ```no_run
/// use quillmark::{Quillmark, Quill, OutputFormat};
///
/// // Step 1: Create engine with auto-registered backends
/// let mut engine = Quillmark::new();
///
/// // Step 2: Create and register quills
/// let quill = Quill::from_path("path/to/quill").unwrap();
/// engine.register_quill(quill);
///
/// // Step 3: Load workflow by quill name
/// let workflow = engine.load("my-quill").unwrap();
///
/// // Step 4: Render markdown
/// let result = workflow.render("# Hello", Some(OutputFormat::Pdf)).unwrap();
/// ```
///
/// ## Loading by Reference
///
/// ```no_run
/// # use quillmark::{Quillmark, Quill};
/// # let mut engine = Quillmark::new();
/// let quill = Quill::from_path("path/to/quill").unwrap();
/// engine.register_quill(quill.clone());
///
/// // Load by name
/// let workflow1 = engine.load("my-quill").unwrap();
///
/// // Load by object (doesn't need to be registered)
/// let workflow2 = engine.load(&quill).unwrap();
/// ```
///
/// ## Inspecting Engine State
///
/// ```no_run
/// # use quillmark::Quillmark;
/// # let engine = Quillmark::new();
/// println!("Available backends: {:?}", engine.registered_backends());
/// println!("Registered quills: {:?}", engine.registered_quills());
/// ```
pub struct Quillmark {
    backends: HashMap<String, Box<dyn Backend>>,
    quills: HashMap<String, Quill>,
}

impl Quillmark {
    /// Create a new Quillmark engine with auto-registered backends.
    ///
    /// Backends are automatically registered based on enabled crate features:
    /// - `typst` (enabled by default) - Typst backend for PDF/SVG rendering
    ///
    /// # Examples
    ///
    /// ```
    /// use quillmark::Quillmark;
    ///
    /// let engine = Quillmark::new();
    /// assert!(engine.registered_backends().len() > 0);
    /// ```
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut backends: HashMap<String, Box<dyn Backend>> = HashMap::new();

        // Auto-register backends based on enabled features
        #[cfg(feature = "typst")]
        {
            let backend = Box::new(quillmark_typst::TypstBackend::default());
            backends.insert(backend.id().to_string(), backend);
        }

        Self {
            backends,
            quills: HashMap::new(),
        }
    }

    /// Register a quill template with the engine.
    ///
    /// Once registered, the quill can be referenced by name when loading workflows.
    /// The quill's name is taken from its metadata (`quill.name`).
    ///
    /// # Arguments
    ///
    /// * `quill` - The quill template to register
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Quill};
    /// let mut engine = Quillmark::new();
    /// let quill = Quill::from_path("path/to/quill").unwrap();
    ///
    /// engine.register_quill(quill);
    /// assert!(engine.registered_quills().contains(&"my-quill"));
    /// ```
    pub fn register_quill(&mut self, quill: Quill) {
        let name = quill.name.clone();
        self.quills.insert(name, quill);
    }

    /// Load a workflow for rendering with a specific quill.
    ///
    /// Accepts either a quill name (as `&str`, `&String`, etc.) or a borrowed [`Quill`] object.
    /// The quill's metadata must specify a `backend` field that matches a registered backend.
    ///
    /// # Arguments
    ///
    /// * `quill_ref` - Reference to a quill, either by name or by object
    ///
    /// # Returns
    ///
    /// Returns a [`Workflow`] configured with the appropriate backend and quill.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError`] if:
    /// - The quill name is not registered (when loading by name)
    /// - The quill doesn't specify a backend in its metadata
    /// - The specified backend is not registered or not enabled
    ///
    /// # Examples
    ///
    /// ## Load by Name
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Quill};
    /// # let mut engine = Quillmark::new();
    /// # let quill = Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill.clone());
    /// let workflow = engine.load("my-quill").unwrap();
    /// ```
    ///
    /// ## Load by Object
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Quill};
    /// # let mut engine = Quillmark::new();
    /// let quill = Quill::from_path("path/to/quill").unwrap();
    /// let workflow = engine.load(&quill).unwrap();
    /// ```
    pub fn load<'a>(&self, quill_ref: impl Into<QuillRef<'a>>) -> Result<Workflow, RenderError> {
        let quill_ref = quill_ref.into();

        // Get the quill reference based on the parameter type
        let quill = match quill_ref {
            QuillRef::Name(name) => {
                // Look up the quill by name
                self.quills.get(name).ok_or_else(|| {
                    RenderError::Other(format!("Quill '{}' not registered", name).into())
                })?
            }
            QuillRef::Object(quill) => {
                // Use the provided quill directly
                quill
            }
        };

        // Get backend ID from quill metadata
        let backend_id = quill
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RenderError::Other(
                    format!("Quill '{}' does not specify a backend", quill.name).into(),
                )
            })?;

        // Get the backend by ID
        let backend = self.backends.get(backend_id).ok_or_else(|| {
            RenderError::Other(
                format!("Backend '{}' not registered or not enabled", backend_id).into(),
            )
        })?;

        // Clone the backend and quill for the workflow
        // Note: We need to box clone the backend trait object
        let backend_clone = self.clone_backend(backend.as_ref());
        let quill_clone = quill.clone();

        Workflow::new(backend_clone, quill_clone)
    }

    /// Helper method to clone a backend (trait object cloning workaround)
    fn clone_backend(&self, backend: &dyn Backend) -> Box<dyn Backend> {
        // For each backend, we need to instantiate a new one
        // This is a workaround since we can't clone trait objects directly
        match backend.id() {
            #[cfg(feature = "typst")]
            "typst" => Box::new(quillmark_typst::TypstBackend::default()),
            _ => panic!("Unknown backend: {}", backend.id()),
        }
    }

    /// Get a list of registered backend IDs.
    ///
    /// Returns the identifiers of all backends that are currently registered
    /// with this engine. Backends are auto-registered based on enabled features.
    ///
    /// # Returns
    ///
    /// A vector of backend ID strings (e.g., `["typst"]`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use quillmark::Quillmark;
    /// let engine = Quillmark::new();
    /// let backends = engine.registered_backends();
    /// println!("Available backends: {:?}", backends);
    /// ```
    pub fn registered_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// Get a list of registered quill names.
    ///
    /// Returns the names of all quills that have been registered with this engine
    /// via [`register_quill`](Self::register_quill).
    ///
    /// # Returns
    ///
    /// A vector of quill name strings.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Quill};
    /// let mut engine = Quillmark::new();
    /// # let quill = Quill::from_path("path/to/quill").unwrap();
    /// engine.register_quill(quill);
    ///
    /// let quills = engine.registered_quills();
    /// println!("Registered quills: {:?}", quills);
    /// ```
    pub fn registered_quills(&self) -> Vec<&str> {
        self.quills.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}
