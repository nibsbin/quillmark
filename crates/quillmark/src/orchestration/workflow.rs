use quillmark_core::{
    normalize_document, Backend, Diagnostic, OutputFormat, ParsedDocument, Plate, Quill,
    RenderError, RenderOptions, RenderResult, Severity,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Sealed workflow for rendering Markdown documents. See [module docs](super) for usage patterns.
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

    /// Render Markdown with YAML frontmatter to output artifacts. See [module docs](super) for examples.
    pub fn render(
        &self,
        parsed: &ParsedDocument,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        let plated_output = self.process_plate(parsed)?;

        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();

        // Pass prepared quill to backend
        self.render_plate_with_quill(&plated_output, format, &prepared_quill)
    }

    /// Render pre-processed plate content, skipping parsing and template composition.
    pub fn render_plate(
        &self,
        content: &str,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        // Prepare quill with dynamic assets
        let prepared_quill = self.prepare_quill_with_assets();
        self.render_plate_with_quill(content, format, &prepared_quill)
    }

    /// Internal method to render content with a specific quill
    fn render_plate_with_quill(
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

    /// Process a parsed document through the plate template without compilation
    ///
    /// Note: Default values from the schema are NOT automatically injected.
    /// This follows JSON Schema semantics where `default` is purely informational.
    /// Typst plates should handle missing optional fields via `.at("field", default: ...)`.
    pub fn process_plate(&self, parsed: &ParsedDocument) -> Result<String, RenderError> {
        // Apply coercion based on schema (no default imputation - aligns with JSON Schema semantics)
        let parsed_coerced = parsed.with_coercion(&self.quill.schema);

        // Validate document against schema
        self.validate_document(&parsed_coerced)?;

        // Normalize document: strip bidi characters and process guillemets
        // - Strips Unicode bidirectional formatting characters that interfere with markdown parsing
        // - Converts <<text>> to «text» in body (guillemets)
        // - Strips chevrons in other fields (<<text>> → text)
        let normalized = normalize_document(parsed_coerced);

        // Create appropriate plate based on whether template is provided
        let mut plate = match &self.quill.plate {
            Some(s) if !s.is_empty() => Plate::new(s.to_string()),
            _ => Plate::new_auto(),
        };
        self.backend.register_filters(&mut plate);
        let plated_output = plate.compose(normalized.fields().clone()).map_err(|e| {
            RenderError::TemplateFailed {
                diag: Box::new(
                    Diagnostic::new(Severity::Error, e.to_string())
                        .with_code("template::compose".to_string()),
                ),
            }
        })?;
        Ok(plated_output)
    }

    /// Perform a dry run validation without backend compilation.
    ///
    /// Executes parsing, schema validation, and template composition to
    /// surface input errors quickly. Returns `Ok(())` on success, or
    /// `Err(RenderError)` with structured diagnostics on failure.
    ///
    /// This is useful for fast feedback loops in LLM-driven document generation,
    /// where you want to validate inputs before incurring compilation costs.
    pub fn dry_run(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
        self.process_plate(parsed)?;
        Ok(())
    }

    /// Validate a ParsedDocument against the Quill's schema
    ///
    /// Validates the document's fields against the schema defined in the Quill.
    /// The schema is built from the TOML `[fields]` section converted to JSON Schema.
    ///
    /// If no schema is defined, this returns Ok(()).
    pub fn validate_schema(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
        self.validate_document(parsed)
    }

    /// Internal validation method
    fn validate_document(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
        use quillmark_core::schema;

        // Build or load JSON Schema

        if self.quill.schema.is_null() {
            // No schema defined, skip validation
            return Ok(());
        };

        // Validate document
        match schema::validate_document(&self.quill.schema, parsed.fields()) {
            Ok(_) => Ok(()),
            Err(errors) => {
                let error_message = errors.join("\n");
                Err(RenderError::ValidationFailed {
                    diag: Box::new(
                        Diagnostic::new(Severity::Error, error_message)
                            .with_code("validation::document_invalid".to_string())
                            .with_hint(
                                "Ensure all required fields are present and have correct types"
                                    .to_string(),
                            ),
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

    /// Add a dynamic asset to the workflow. See [module docs](super) for examples.
    pub fn add_asset(
        &mut self,
        filename: impl Into<String>,
        contents: impl Into<Vec<u8>>,
    ) -> Result<(), RenderError> {
        let filename = filename.into();

        // Check for collision
        if self.dynamic_assets.contains_key(&filename) {
            return Err(RenderError::DynamicAssetCollision {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!(
                        "Dynamic asset '{}' already exists. Each asset filename must be unique.",
                        filename
                    ),
                    )
                    .with_code("workflow::asset_collision".to_string())
                    .with_hint("Use unique filenames for each dynamic asset".to_string()),
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
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Dynamic font '{}' already exists. Each font filename must be unique.",
                            filename
                        ),
                    )
                    .with_code("workflow::font_collision".to_string())
                    .with_hint("Use unique filenames for each dynamic font".to_string()),
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
