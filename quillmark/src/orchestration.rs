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
    Backend, Diagnostic, Glue, OutputFormat, ParsedDocument, Quill, RenderError, RenderOptions,
    RenderResult, Severity,
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
    /// #     fn glue_extension_types(&self) -> &'static [&'static str] { &[".custom"] }
    /// #     fn allow_auto_glue(&self) -> bool { true }
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
    ///
    /// Validates the quill configuration against the registered backend, including:
    /// - Backend exists and is registered
    /// - Glue file extension matches backend requirements
    /// - Auto-glue is allowed if no glue file is specified
    /// - Quill name is unique
    pub fn register_quill(&mut self, quill: Quill) -> Result<(), RenderError> {
        let name = quill.name.clone();

        // Check name uniqueness
        if self.quills.contains_key(&name) {
            return Err(RenderError::QuillConfig {
                diag: Diagnostic::new(
                    Severity::Error,
                    format!("Quill '{}' is already registered", name),
                )
                .with_code("quill::name_collision".to_string())
                .with_hint("Each quill must have a unique name".to_string()),
            });
        }

        // Get backend
        let backend_id = quill.backend.as_str();
        let backend = self
            .backends
            .get(backend_id)
            .ok_or_else(|| RenderError::QuillConfig {
                diag: Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Backend '{}' specified in quill '{}' is not registered",
                        backend_id, name
                    ),
                )
                .with_code("quill::backend_not_found".to_string())
                .with_hint(format!(
                    "Available backends: {}",
                    self.backends.keys().cloned().collect::<Vec<_>>().join(", ")
                )),
            })?;

        // Validate glue_file extension or auto_glue
        if let Some(glue_file) = &quill.metadata.get("glue_file").and_then(|v| v.as_str()) {
            let extension = std::path::Path::new(glue_file)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();

            if !backend.glue_extension_types().contains(&extension.as_str()) {
                return Err(RenderError::QuillConfig {
                    diag: Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Glue file '{}' has extension '{}' which is not supported by backend '{}'",
                            glue_file, extension, backend_id
                        ),
                    )
                    .with_code("quill::glue_extension_mismatch".to_string())
                    .with_hint(format!(
                        "Supported extensions for '{}' backend: {}",
                        backend_id,
                        backend.glue_extension_types().join(", ")
                    )),
                });
            }
        } else {
            if !backend.allow_auto_glue() {
                return Err(RenderError::QuillConfig {
                    diag: Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Backend '{}' does not support automatic glue generation, but quill '{}' does not specify a glue file",
                            backend_id, name
                        ),
                    )
                    .with_code("quill::auto_glue_not_allowed".to_string())
                    .with_hint(format!(
                        "Add a glue file with one of these extensions: {}",
                        backend.glue_extension_types().join(", ")
                    )),
                });
            }
        }

        self.quills.insert(name, quill);
        Ok(())
    }

    /// Load a workflow from a parsed document that contains a quill tag
    pub fn workflow_from_parsed(&self, parsed: &ParsedDocument) -> Result<Workflow, RenderError> {
        let quill_name = parsed.quill_tag().ok_or_else(|| {
            RenderError::UnsupportedBackend {
                diag: Diagnostic::new(
                    Severity::Error,
                    "No QUILL field found in parsed document. Add `QUILL: <name>` to the markdown frontmatter.".to_string(),
                )
                .with_code("engine::missing_quill_tag".to_string())
                .with_hint("Add QUILL: <name> to specify which quill template to use".to_string()),
            }
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
                self.quills
                    .get(name)
                    .ok_or_else(|| RenderError::UnsupportedBackend {
                        diag: Diagnostic::new(
                            Severity::Error,
                            format!("Quill '{}' not registered", name),
                        )
                        .with_code("engine::quill_not_found".to_string())
                        .with_hint(format!(
                            "Available quills: {}",
                            self.quills.keys().cloned().collect::<Vec<_>>().join(", ")
                        )),
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
            .ok_or_else(|| RenderError::EngineCreation {
                diag: Diagnostic::new(
                    Severity::Error,
                    format!("Quill '{}' does not specify a backend", quill.name),
                )
                .with_code("engine::missing_backend".to_string())
                .with_hint(
                    "Add 'backend = \"typst\"' to the [Quill] section of Quill.toml".to_string(),
                ),
            })?;

        // Get the backend by ID
        let backend =
            self.backends
                .get(backend_id)
                .ok_or_else(|| RenderError::UnsupportedBackend {
                    diag: Diagnostic::new(
                        Severity::Error,
                        format!("Backend '{}' not registered or not enabled", backend_id),
                    )
                    .with_code("engine::backend_not_found".to_string())
                    .with_hint(format!(
                        "Available backends: {}",
                        self.backends.keys().cloned().collect::<Vec<_>>().join(", ")
                    )),
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
        let glue_output = self.process_glue(parsed)?;

        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();

        // Pass prepared quill to backend
        self.render_processed_with_quill(&glue_output, format, &prepared_quill)
    }

    /// Render pre-processed glue content, skipping parsing and template composition.
    pub fn render_processed(
        &self,
        content: &str,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();
        self.render_processed_with_quill(content, format, &prepared_quill)
    }

    /// Internal method to render content with a specific quill
    fn render_processed_with_quill(
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

    /// Process a parsed document through the glue template without compilation
    pub fn process_glue(&self, parsed: &ParsedDocument) -> Result<String, RenderError> {
        // Apply defaults from field schemas
        let parsed_with_defaults = parsed.with_defaults(&self.quill.field_schemas);

        // Validate document against schema
        self.validate_document(&parsed_with_defaults)?;

        // Create appropriate glue based on whether template is provided
        let mut glue = match &self.quill.glue {
            Some(s) if !s.is_empty() => Glue::new(s.to_string()),
            _ => Glue::new_auto(),
        };
        self.backend.register_filters(&mut glue);
        let glue_output = glue
            .compose(parsed_with_defaults.fields().clone())
            .map_err(|e| RenderError::TemplateFailed {
                diag: Diagnostic::new(Severity::Error, e.to_string())
                    .with_code("template::compose".to_string()),
            })?;
        Ok(glue_output)
    }

    /// Validate a ParsedDocument against the Quill's schema
    ///
    /// Validates the document's fields against the schema defined in the Quill.
    /// The schema can come from either:
    /// - A json_schema_file specified in Quill.toml
    /// - TOML `[fields]` section converted to JSON Schema
    ///
    /// If no schema is defined, this returns Ok(()).
    pub fn validate(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
        self.validate_document(parsed)
    }

    /// Internal validation method
    fn validate_document(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
        use quillmark_core::validation;

        // Build or load JSON Schema

        if self.quill.schema.is_null() {
            // No schema defined, skip validation
            return Ok(());
        };

        // Validate document
        match validation::validate_document(&self.quill.schema, parsed.fields()) {
            Ok(_) => Ok(()),
            Err(errors) => {
                let error_message = errors.join("\n");
                Err(RenderError::ValidationFailed {
                    diag: Diagnostic::new(Severity::Error, error_message)
                        .with_code("validation::document_invalid".to_string())
                        .with_hint(
                            "Ensure all required fields are present and have correct types"
                                .to_string(),
                        ),
                })
            }
        }
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
                diag: Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Dynamic asset '{}' already exists. Each asset filename must be unique.",
                        filename
                    ),
                )
                .with_code("workflow::asset_collision".to_string())
                .with_hint("Use unique filenames for each dynamic asset".to_string()),
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
                diag: Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Dynamic font '{}' already exists. Each font filename must be unique.",
                        filename
                    ),
                )
                .with_code("workflow::font_collision".to_string())
                .with_hint("Use unique filenames for each dynamic font".to_string()),
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
