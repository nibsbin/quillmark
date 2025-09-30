// Re-export all core types for backward compatibility
pub use quillmark_core::{
    Artifact, Backend, OutputFormat, Quill, 
    RenderError, RenderResult, Diagnostic, Severity, Location,
    decompose, ParsedDocument, BODY_FIELD, Glue, TemplateError
};

use quillmark_core::{RenderOptions};

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
    pub fn render(&self, markdown: &str, format: Option<OutputFormat>) -> Result<RenderResult, RenderError> {
        let glue_output = self.process_glue(markdown)?;
        let rendered = self.render_content(&glue_output, format)?;
        Ok(rendered)
    }

    /// Render pre-processed glue content to a specific output format
    pub fn render_content(&self, content: &str, mut format: Option<OutputFormat>) -> Result<RenderResult, RenderError> {
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
        let parsed_doc = decompose(markdown)
            .map_err(|e| RenderError::InvalidFrontmatter {
                diag: quillmark_core::error::Diagnostic::new(
                    quillmark_core::error::Severity::Error,
                    format!("Failed to parse markdown: {}", e)
                ),
                source: Some(anyhow::anyhow!(e))
            })?;

        let mut glue = Glue::new(self.quill.glue_template.clone());
        self.backend.register_filters(&mut glue);
        let glue_output = glue.compose(parsed_doc.fields().clone())
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