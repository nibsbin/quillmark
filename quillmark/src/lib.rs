// Re-export all core types for backward compatibility
pub use quillmark_core::{
    decompose, Artifact, Backend, Diagnostic, Glue, Location, OutputFormat, ParsedDocument, Quill,
    RenderError, RenderResult, Severity, TemplateError, BODY_FIELD,
};

use quillmark_core::RenderOptions;
use std::collections::HashMap;

/// Reference to a Quill, either by name or by borrowed object
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

/// The main sealed engine API
pub struct Workflow {
    backend: Box<dyn Backend>,
    quill: Quill,
}

impl Workflow {
    /// Create a new Workflow with the specified backend and quill template
    pub fn new(backend: Box<dyn Backend>, quill: Quill) -> Result<Self, RenderError> {
        // Since Quill::from_path() now automatically validates, we don't need to validate again
        Ok(Self { backend, quill })
    }

    /// Render markdown to a specific output format
    pub fn render(
        &self,
        markdown: &str,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        let glue_output = self.process_glue(markdown)?;
        let rendered = self.render_content(&glue_output, format)?;
        Ok(rendered)
    }

    /// Render processed glue content to a specific output format
    pub fn render_content(
        &self,
        content: &str,
        mut format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        // Compile using backend
        if !format.is_some() {
            // Default to first supported format if none specified
            let supported = self.backend.supported_formats();
            if !supported.is_empty() {
                println!("Defaulting to output format: {:?}", supported[0]);
                format = Some(supported[0]);
            }
        }
        // Compile using backend
        let render_opts = RenderOptions {
            output_format: format,
        };

        let artifacts = self.backend.compile(content, &self.quill, &render_opts)?;
        Ok(RenderResult::new(artifacts))
    }

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

    /// Get the backend ID
    pub fn backend_id(&self) -> &str {
        self.backend.id()
    }

    /// Get supported output formats
    pub fn supported_formats(&self) -> &'static [OutputFormat] {
        self.backend.supported_formats()
    }

    /// Get the quill name
    pub fn quill_name(&self) -> &str {
        &self.quill.name
    }
}

/// High-level engine for orchestrating backends and quills
///
/// `Quillmark` manages the registration of backends and quills, and provides
/// a convenient way to create workflows. Backends are automatically registered
/// based on enabled crate features.
///
/// # Example
///
/// ```no_run
/// use quillmark::{Quillmark, Quill, OutputFormat};
///
/// // Step 1: Create engine with auto-registered backends (typst by default)
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
pub struct Quillmark {
    backends: HashMap<String, Box<dyn Backend>>,
    quills: HashMap<String, Quill>,
}

impl Quillmark {
    /// Create a new Quillmark with auto-registered backends based on enabled features
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

    /// Register a quill by name
    pub fn register_quill(&mut self, quill: Quill) {
        let name = quill.name.clone();
        self.quills.insert(name, quill);
    }

    /// Load a workflow for a quill
    ///
    /// Accepts either a quill name (as &str, &String, etc.) or a borrowed Quill object.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Quill};
    /// # let mut engine = Quillmark::new();
    /// # let quill = Quill::from_path("path/to/quill").unwrap();
    /// # engine.register_quill(quill.clone());
    /// // Load by name
    /// let workflow = engine.load("my-quill").unwrap();
    ///
    /// // Load by object
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

    /// Get list of registered backend IDs
    pub fn registered_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// Get list of registered quill names
    pub fn registered_quills(&self) -> Vec<&str> {
        self.quills.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}
