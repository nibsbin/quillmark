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
//! 3. Load workflows with [`Quillmark::workflow_from_quill_name()`] or [`Quillmark::workflow_from_parsed()`]
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
//! // Step 4: Load workflow by quill name and render
//! let workflow = engine.workflow_from_quill_name("my-quill").unwrap();
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
//! let workflow1 = engine.workflow_from_quill_name("my-quill").unwrap();
//!
//! // Load by object (doesn't need to be registered)
//! let workflow2 = engine.workflow_from_quill(&quill).unwrap();
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
//! 2. **Content render** ([`Workflow::render_source()`]) - Skip parsing, render pre-composed content
//! 3. **Glue only** ([`Workflow::process_glue_parsed()`]) - Compose from parsed document, return template output
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
//! let workflow = engine.workflow_from_quill_name("my-quill").unwrap();
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
//! let mut workflow = engine.workflow_from_quill_name("my-quill").unwrap();
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
//! let mut workflow = engine.workflow_from_quill_name("my-quill").unwrap();
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
//! let workflow = engine.workflow_from_quill_name("my-quill").unwrap();
//!
//! println!("Backend: {}", workflow.backend_id());
//! println!("Quill: {}", workflow.quill_name());
//! println!("Formats: {:?}", workflow.supported_formats());
//! ```

use quillmark_core::{
    decompose, Backend, Glue, OutputFormat, ParsedDocument, Quill, RenderError, RenderOptions,
    RenderResult,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Ergonomic reference to a Quill by name or object.
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

/// High-level engine for orchestrating backends and quills. See [module docs](self) for usage patterns.
pub struct Quillmark {
    backends: HashMap<String, Arc<dyn Backend>>,
    quills: HashMap<String, Quill>,
}

impl Quillmark {
    /// Create a new Quillmark with auto-registered backends based on enabled features.
    pub fn new() -> Self {
        let mut engine = Self {
            backends: HashMap::new(),
            quills: HashMap::new(),
        };

        // Auto-register backends based on enabled features
        #[cfg(feature = "typst")]
        {
            engine.register_backend(Box::new(quillmark_typst::TypstBackend));
        }

        #[cfg(feature = "acroform")]
        {
            engine.register_backend(Box::new(quillmark_acroform::AcroformBackend));
        }

        engine
    }

    /// Register a backend with the engine.
    ///
    /// This method allows registering custom backends or explicitly registering
    /// feature-integrated backends. The backend is registered by its ID.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use quillmark::Quillmark;
    /// # use quillmark_core::Backend;
    /// # struct CustomBackend;
    /// # impl Backend for CustomBackend {
    /// #     fn id(&self) -> &'static str { "custom" }
    /// #     fn supported_formats(&self) -> &'static [quillmark_core::OutputFormat] { &[] }
    /// #     fn glue_type(&self) -> &'static str { ".custom" }
    /// #     fn register_filters(&self, _: &mut quillmark_core::Glue) {}
    /// #     fn compile(&self, _: &str, _: &quillmark_core::Quill, _: &quillmark_core::RenderOptions) -> Result<quillmark_core::RenderResult, quillmark_core::RenderError> {
    /// #         Ok(quillmark_core::RenderResult::new(vec![], quillmark_core::OutputFormat::Txt))
    /// #     }
    /// # }
    ///
    /// let mut engine = Quillmark::new();
    /// let custom_backend = Box::new(CustomBackend);
    /// engine.register_backend(custom_backend);
    /// ```
    pub fn register_backend(&mut self, backend: Box<dyn Backend>) {
        let id = backend.id().to_string();
        self.backends.insert(id, Arc::from(backend));
    }

    /// Register a quill template with the engine by name.
    pub fn register_quill(&mut self, quill: Quill) {
        let name = quill.name.clone();
        self.quills.insert(name, quill);
    }

    /// Load a workflow from a parsed document that contains a quill tag
    pub fn workflow_from_parsed(&self, parsed: &ParsedDocument) -> Result<Workflow, RenderError> {
        let quill_name = parsed.quill_tag().ok_or_else(|| {
            RenderError::Other(
                "No QUILL field found in parsed document. Add `QUILL: <name>` to the markdown frontmatter.".into(),
            )
        })?;
        self.workflow_from_quill_name(quill_name)
    }

    /// Load a workflow by quill reference (name or object)
    pub fn workflow_from_quill<'a>(
        &self,
        quill_ref: impl Into<QuillRef<'a>>,
    ) -> Result<Workflow, RenderError> {
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

        // Clone the Arc reference to the backend and the quill for the workflow
        let backend_clone = Arc::clone(backend);
        let quill_clone = quill.clone();

        Workflow::new(backend_clone, quill_clone)
    }

    /// Load a workflow by quill name
    pub fn workflow_from_quill_name(&self, name: &str) -> Result<Workflow, RenderError> {
        self.workflow_from_quill(name)
    }

    /// Get a list of registered backend IDs.
    pub fn registered_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// Get a list of registered quill names.
    pub fn registered_quills(&self) -> Vec<&str> {
        self.quills.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}

/// Sealed workflow for rendering Markdown documents. See [module docs](self) for usage patterns.
pub struct Workflow {
    backend: Arc<dyn Backend>,
    quill: Quill,
    dynamic_assets: HashMap<String, Vec<u8>>,
    dynamic_fonts: HashMap<String, Vec<u8>>,
}

impl Workflow {
    /// Create a new Workflow with the specified backend and quill.
    pub fn new(backend: Arc<dyn Backend>, quill: Quill) -> Result<Self, RenderError> {
        // Since Quill::from_path() now automatically validates, we don't need to validate again
        Ok(Self {
            backend,
            quill,
            dynamic_assets: HashMap::new(),
            dynamic_fonts: HashMap::new(),
        })
    }

    /// Render Markdown with YAML frontmatter to output artifacts. See [module docs](self) for examples.
    pub fn render(
        &self,
        parsed: &ParsedDocument,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        let glue_output = self.process_glue_parsed(parsed)?;

        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();

        // Pass prepared quill to backend
        self.render_source_with_quill(&glue_output, format, &prepared_quill)
    }

    /// Render pre-processed glue content, skipping parsing and template composition.
    pub fn render_source(
        &self,
        content: &str,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();
        self.render_source_with_quill(content, format, &prepared_quill)
    }

    /// Internal method to render content with a specific quill
    fn render_source_with_quill(
        &self,
        content: &str,
        format: Option<OutputFormat>,
        quill: &Quill,
    ) -> Result<RenderResult, RenderError> {
        let format = if format.is_some() {
            format
        } else {
            // Default to first supported format if none specified
            let supported = self.backend.supported_formats();
            if !supported.is_empty() {
                Some(supported[0])
            } else {
                None
            }
        };

        let render_opts = RenderOptions {
            output_format: format,
        };

        self.backend.compile(content, quill, &render_opts)
    }

    /// Process Markdown through the glue template without compilation, returning the composed output.
    pub fn process_glue(&self, markdown: &str) -> Result<String, RenderError> {
        let parsed_doc = decompose(markdown).map_err(|e| RenderError::InvalidFrontmatter {
            diag: quillmark_core::error::Diagnostic::new(
                quillmark_core::error::Severity::Error,
                format!("Failed to parse markdown: {}", e),
            ),
            source: Some(anyhow::anyhow!(e)),
        })?;

        self.process_glue_parsed(&parsed_doc)
    }

    /// Process a parsed document through the glue template without compilation
    pub fn process_glue_parsed(&self, parsed: &ParsedDocument) -> Result<String, RenderError> {
        // Create appropriate glue based on whether template is provided
        let mut glue = if self.quill.glue_template.is_empty() {
            Glue::new_json()
        } else {
            Glue::new(self.quill.glue_template.clone())
        };

        self.backend.register_filters(&mut glue);
        let glue_output = glue
            .compose(parsed.fields().clone())
            .map_err(RenderError::from)?;
        Ok(glue_output)
    }

    /// Get the backend identifier (e.g., "typst").
    pub fn backend_id(&self) -> &str {
        self.backend.id()
    }

    /// Get the supported output formats for this workflow's backend.
    pub fn supported_formats(&self) -> &'static [OutputFormat] {
        self.backend.supported_formats()
    }

    /// Get the quill name used by this workflow.
    pub fn quill_name(&self) -> &str {
        &self.quill.name
    }

    /// Return the list of dynamic asset filenames currently stored in the workflow.
    ///
    /// This is primarily a debugging helper so callers (for example wasm bindings)
    /// can inspect which assets have been added via `add_asset` / `add_assets`.
    pub fn dynamic_asset_names(&self) -> Vec<String> {
        self.dynamic_assets.keys().cloned().collect()
    }

    /// Add a dynamic asset to the workflow. See [module docs](self) for examples.
    pub fn add_asset(
        &mut self,
        filename: impl Into<String>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<(), RenderError> {
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
        Ok(())
    }

    /// Add multiple dynamic assets at once.
    pub fn add_assets(
        &mut self,
        assets: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<(), RenderError> {
        for (filename, contents) in assets {
            self.add_asset(filename, contents)?;
        }
        Ok(())
    }

    /// Clear all dynamic assets from the workflow.
    pub fn clear_assets(&mut self) {
        self.dynamic_assets.clear();
    }

    /// Return the list of dynamic font filenames currently stored in the workflow.
    ///
    /// This is primarily a debugging helper so callers (for example wasm bindings)
    /// can inspect which fonts have been added via `add_font` / `add_fonts`.
    pub fn dynamic_font_names(&self) -> Vec<String> {
        self.dynamic_fonts.keys().cloned().collect()
    }

    /// Add a dynamic font to the workflow. Fonts are saved to assets/ with DYNAMIC_FONT__ prefix.
    pub fn add_font(
        &mut self,
        filename: impl Into<String>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<(), RenderError> {
        let filename = filename.into();

        // Check for collision
        if self.dynamic_fonts.contains_key(&filename) {
            return Err(RenderError::DynamicFontCollision {
                filename: filename.clone(),
                message: format!(
                    "Dynamic font '{}' already exists. Each font filename must be unique.",
                    filename
                ),
            });
        }

        self.dynamic_fonts.insert(filename, contents.into());
        Ok(())
    }

    /// Add multiple dynamic fonts at once.
    pub fn add_fonts(
        &mut self,
        fonts: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<(), RenderError> {
        for (filename, contents) in fonts {
            self.add_font(filename, contents)?;
        }
        Ok(())
    }

    /// Clear all dynamic fonts from the workflow.
    pub fn clear_fonts(&mut self) {
        self.dynamic_fonts.clear();
    }

    /// Internal method to prepare a quill with dynamic assets and fonts
    fn prepare_quill_with_assets(&self) -> Quill {
        use quillmark_core::FileTreeNode;

        let mut quill = self.quill.clone();

        // Add dynamic assets to the cloned quill's file system
        for (filename, contents) in &self.dynamic_assets {
            let prefixed_path = format!("assets/DYNAMIC_ASSET__{}", filename);
            let file_node = FileTreeNode::File {
                contents: contents.clone(),
            };
            // Ignore errors if insertion fails (e.g., path already exists)
            let _ = quill.files.insert(&prefixed_path, file_node);
        }

        // Add dynamic fonts to the cloned quill's file system
        for (filename, contents) in &self.dynamic_fonts {
            let prefixed_path = format!("assets/DYNAMIC_FONT__{}", filename);
            let file_node = FileTreeNode::File {
                contents: contents.clone(),
            };
            // Ignore errors if insertion fails (e.g., path already exists)
            let _ = quill.files.insert(&prefixed_path, file_node);
        }

        quill
    }
}
