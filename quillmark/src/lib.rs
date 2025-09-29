use std::path::PathBuf;

// Re-export all core types for backward compatibility
pub use quillmark_core::{
    Artifact, Backend, OutputFormat, Quill, 
    RenderError, RenderResult, Diagnostic, Severity, Location,
    decompose, ParsedDocument, BODY_FIELD, Glue, TemplateError
};

use quillmark_core::{RenderOptions};

/// The main sealed engine API
pub struct QuillEngine {
    backend: Box<dyn Backend>,
    quill: Quill,
}

impl QuillEngine {
    /// Create a new QuillEngine with the specified backend and quill template
    pub fn new(backend: Box<dyn Backend>, quill_path: PathBuf) -> Result<Self, RenderError> {
        // Load the quill template
        let quill = Quill::from_path(&quill_path)
            .map_err(|e| RenderError::EngineCreation { 
                diag: quillmark_core::error::Diagnostic::new(
                    quillmark_core::error::Severity::Error,
                    format!("Failed to load quill from {:?}: {}", quill_path, e)
                ),
                source: Some(anyhow::anyhow!(e))
            })?;

        // Validate the quill
        quill.validate()
            .map_err(|e| RenderError::EngineCreation { 
                diag: quillmark_core::error::Diagnostic::new(
                    quillmark_core::error::Severity::Error,
                    format!("Quill validation failed: {}", e)
                ),
                source: Some(anyhow::anyhow!(e))
            })?;

        Ok(Self { backend, quill })
    }

    /// Render markdown using the engine's backend and quill template
    pub fn render(&self, markdown: &str) -> Result<RenderResult, RenderError> {
        self.render_with_format(markdown, None)
    }

    /// Render markdown with a specific output format
    pub fn render_with_format(&self, markdown: &str, format: Option<OutputFormat>) -> Result<RenderResult, RenderError> {
        // Step 1: Parse markdown into frontmatter and body
        let parsed_doc = decompose(markdown)
            .map_err(|e| RenderError::InvalidFrontmatter {
                diag: quillmark_core::error::Diagnostic::new(
                    quillmark_core::error::Severity::Error,
                    format!("Failed to parse markdown: {}", e)
                ),
                source: Some(anyhow::anyhow!(e))
            })?;

        // Step 2: Setup glue with template content
        let mut glue = Glue::new(self.quill.template_content.clone());

        // Step 3: Register backend filters
        self.backend.register_filters(&mut glue);

        // Step 4: Compose template with parsed context
        let glue_content = glue.compose(parsed_doc.fields().clone())
            .map_err(|e| RenderError::from(e))?;

        // Step 5: Compile using backend
        let render_opts = RenderOptions {
            output_format: format,
        };

        let artifacts = self.backend.compile(&glue_content, &self.quill, &render_opts)?;
        Ok(RenderResult::new(artifacts))
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