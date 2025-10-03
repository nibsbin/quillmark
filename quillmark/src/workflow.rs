//! Sealed workflow for rendering Markdown documents. See [module docs](self) for usage patterns.

#![doc = include_str!("../docs/workflow.md")]

use quillmark_core::{
    decompose, Backend, Glue, OutputFormat, Quill, RenderError, RenderResult, RenderOptions,
};
use std::collections::HashMap;

/// Sealed workflow for rendering Markdown documents. See [module docs](self) for usage patterns.
#[doc = include_str!("../docs/workflow.md")]
pub struct Workflow {
    backend: Box<dyn Backend>,
    quill: Quill,
    dynamic_assets: HashMap<String, Vec<u8>>,
}

impl Workflow {
    /// Create a new Workflow with the specified backend and quill. Usually called via [`crate::Quillmark::load`].
    pub fn new(backend: Box<dyn Backend>, quill: Quill) -> Result<Self, RenderError> {
        // Since Quill::from_path() now automatically validates, we don't need to validate again
        Ok(Self {
            backend,
            quill,
            dynamic_assets: HashMap::new(),
        })
    }

    /// Render Markdown with YAML frontmatter to output artifacts. See [module docs](self) for examples.
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

    /// Render pre-processed glue content, skipping parsing and template composition.
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

    /// Process Markdown through the glue template without compilation, returning the composed output.
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

    /// Add a dynamic asset to the workflow (builder pattern). See [module docs](self) for examples.
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
