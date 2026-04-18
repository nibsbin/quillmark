use quillmark_core::{
    normalize::normalize_document, quill::FieldSchema, quill::FieldType, Backend, CompiledDocument,
    Diagnostic, OutputFormat, ParsedDocument, Quill, QuillValue, RenderError, RenderOptions,
    RenderResult, Severity,
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

struct PreparedRenderContext {
    json_data: serde_json::Value,
    plate_content: String,
    prepared_quill: Quill,
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
    /// Compile the document to JSON data suitable for the backend
    pub fn compile_data(&self, parsed: &ParsedDocument) -> Result<serde_json::Value, RenderError> {
        let coerced_fields = self
            .quill
            .config
            .coerce(parsed.fields())
            .map_err(|e| RenderError::ValidationFailed {
                diag: Box::new(
                    Diagnostic::new(Severity::Error, e.to_string())
                        .with_code("validation::coercion_failed".to_string())
                        .with_hint(
                            "Ensure all fields and card values can be coerced to their declared types"
                                .to_string(),
                        ),
                ),
            })?;
        let parsed_coerced = ParsedDocument::new(coerced_fields, parsed.quill_reference().clone());
        self.validate_document(&parsed_coerced)?;

        // Normalize document: strip bidi characters and fix HTML comment fences
        let normalized = normalize_document(parsed_coerced)?;

        // Transform fields for JSON injection (backend-specific transformations)
        let transformed_fields = self
            .backend
            .transform_fields(normalized.fields(), &self.transform_schema());

        // Apply schema defaults to fill in missing fields
        let fields_with_defaults = self.apply_schema_defaults(&transformed_fields);

        // Serialize transformed fields to JSON for injection
        Ok(Self::fields_to_json(&fields_with_defaults))
    }

    pub fn render(
        &self,
        parsed: &ParsedDocument,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        self.render_with_options(parsed, format, None)
    }

    /// Compile parsed data into a backend-specific compiled document for selective page rendering.
    pub fn compile(&self, parsed: &ParsedDocument) -> Result<CompiledDocument, RenderError> {
        let context = self.prepare_render_context(parsed)?;
        self.backend.compile_to_document(
            &context.plate_content,
            &context.prepared_quill,
            &context.json_data,
        )
    }

    /// Render selected pages from a compiled document.
    ///
    /// - `pages = None` renders all pages.
    /// - `pages = Some(&[])` renders zero artifacts.
    pub fn render_pages(
        &self,
        doc: &CompiledDocument,
        pages: Option<&[usize]>,
        format: OutputFormat,
        ppi: Option<f32>,
    ) -> Result<RenderResult, RenderError> {
        self.backend.render_pages(doc, pages, format, ppi)
    }

    /// Render with explicit pixels-per-inch for raster formats (PNG).
    ///
    /// `ppi` is ignored for vector/document formats (PDF, SVG, TXT).
    /// When `None`, defaults to 144.0 (2x at 72pt/inch).
    pub fn render_with_options(
        &self,
        parsed: &ParsedDocument,
        format: Option<OutputFormat>,
        ppi: Option<f32>,
    ) -> Result<RenderResult, RenderError> {
        let context = self.prepare_render_context(parsed)?;

        // Pass plate content and JSON data to backend
        self.render_plate_with_quill_and_data(
            &context.plate_content,
            format,
            ppi,
            &context.prepared_quill,
            &context.json_data,
        )
    }

    fn prepare_render_context(
        &self,
        parsed: &ParsedDocument,
    ) -> Result<PreparedRenderContext, RenderError> {
        Ok(PreparedRenderContext {
            json_data: self.compile_data(parsed)?,
            plate_content: self.get_plate_content()?.unwrap_or_default(),
            prepared_quill: self.prepare_quill_with_assets(),
        })
    }

    /// Internal method to render content with a specific quill and JSON data
    fn render_plate_with_quill_and_data(
        &self,
        content: &str,
        format: Option<OutputFormat>,
        ppi: Option<f32>,
        quill: &Quill,
        json_data: &serde_json::Value,
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
            ppi,
        };

        self.backend
            .compile(content, quill, &render_opts, json_data)
    }

    /// Apply defaults from QuillConfig to fill missing fields
    fn apply_schema_defaults(
        &self,
        fields: &HashMap<String, quillmark_core::QuillValue>,
    ) -> HashMap<String, quillmark_core::QuillValue> {
        let mut result = fields.clone();

        for (field_name, default_value) in self.quill.config.defaults() {
            if !result.contains_key(&field_name) {
                result.insert(field_name, default_value);
            }
        }

        result
    }

    /// Convert fields to JSON Value for injection
    fn fields_to_json(fields: &HashMap<String, quillmark_core::QuillValue>) -> serde_json::Value {
        let mut json_map = serde_json::Map::new();
        for (key, value) in fields {
            json_map.insert(key.clone(), value.as_json().clone());
        }
        serde_json::Value::Object(json_map)
    }

    fn transform_schema(&self) -> QuillValue {
        fn field_to_schema(field: &FieldSchema) -> serde_json::Value {
            let mut schema = serde_json::Map::new();
            match field.r#type {
                FieldType::String => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("string".to_string()),
                    );
                }
                FieldType::Markdown => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("string".to_string()),
                    );
                    schema.insert(
                        "contentMediaType".to_string(),
                        serde_json::Value::String("text/markdown".to_string()),
                    );
                }
                FieldType::Number => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("number".to_string()),
                    );
                }
                FieldType::Integer => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("integer".to_string()),
                    );
                }
                FieldType::Boolean => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("boolean".to_string()),
                    );
                }
                FieldType::Array => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("array".to_string()),
                    );
                    if let Some(items) = &field.items {
                        schema.insert("items".to_string(), field_to_schema(items));
                    }
                }
                FieldType::Object => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("object".to_string()),
                    );
                    if let Some(properties) = &field.properties {
                        let mut props: serde_json::Map<String, serde_json::Value> =
                            serde_json::Map::new();
                        for (name, prop) in properties {
                            props.insert(name.clone(), field_to_schema(prop));
                        }
                        schema.insert("properties".to_string(), serde_json::Value::Object(props));
                    }
                }
                FieldType::Date => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("string".to_string()),
                    );
                    schema.insert(
                        "format".to_string(),
                        serde_json::Value::String("date".to_string()),
                    );
                }
                FieldType::DateTime => {
                    schema.insert(
                        "type".to_string(),
                        serde_json::Value::String("string".to_string()),
                    );
                    schema.insert(
                        "format".to_string(),
                        serde_json::Value::String("date-time".to_string()),
                    );
                }
            }
            serde_json::Value::Object(schema)
        }

        let mut properties = serde_json::Map::new();
        for (name, field) in &self.quill.config.main().fields {
            properties.insert(name.clone(), field_to_schema(field));
        }
        properties.insert(
            "BODY".to_string(),
            serde_json::json!({ "type": "string", "contentMediaType": "text/markdown" }),
        );

        let mut defs = serde_json::Map::new();
        for card in self.quill.config.card_definitions() {
            let mut card_properties = serde_json::Map::new();
            for (name, field) in &card.fields {
                card_properties.insert(name.clone(), field_to_schema(field));
            }
            defs.insert(
                format!("{}_card", card.name),
                serde_json::json!({
                    "type": "object",
                    "properties": card_properties,
                }),
            );
        }

        QuillValue::from_json(serde_json::json!({
            "type": "object",
            "properties": properties,
            "$defs": defs,
        }))
    }

    /// Get the plate content directly from the quill
    ///
    /// Returns the plate file content as-is, without any MiniJinja processing.
    /// Returns None if no plate file exists (valid for pure-binary backends).
    fn get_plate_content(&self) -> Result<Option<String>, RenderError> {
        match &self.quill.plate {
            Some(s) if !s.is_empty() => Ok(Some(s.clone())),
            _ => Ok(None),
        }
    }

    /// Perform a dry run validation without backend compilation.
    ///
    /// Executes parsing and schema validation to surface input errors quickly.
    /// Returns `Ok(())` on success, or `Err(RenderError)` with structured
    /// diagnostics on failure.
    ///
    /// This is useful for fast feedback loops in LLM-driven document generation,
    /// where you want to validate inputs before incurring compilation costs.
    pub fn dry_run(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
        let coerced_fields = self
            .quill
            .config
            .coerce(parsed.fields())
            .map_err(|e| RenderError::ValidationFailed {
                diag: Box::new(
                    Diagnostic::new(Severity::Error, e.to_string())
                        .with_code("validation::coercion_failed".to_string())
                        .with_hint(
                            "Ensure all fields and card values can be coerced to their declared types"
                                .to_string(),
                        ),
                ),
            })?;
        let parsed_coerced = ParsedDocument::new(coerced_fields, parsed.quill_reference().clone());
        self.validate_document(&parsed_coerced)?;
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
        match self.quill.config.validate(parsed.fields()) {
            Ok(_) => Ok(()),
            Err(errors) => {
                let error_message = errors
                    .into_iter()
                    .map(|e| format!("- {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
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

    /// Get a reference-counted handle to the backend.
    pub fn backend(&self) -> Arc<dyn Backend> {
        Arc::clone(&self.backend)
    }

    /// Get the backend identifier (e.g., "typst").
    pub fn backend_id(&self) -> &str {
        self.backend.id()
    }

    /// Get the supported output formats for this workflow's backend.
    pub fn supported_formats(&self) -> &'static [OutputFormat] {
        self.backend.supported_formats()
    }

    /// Get the quill reference (name@version) used by this workflow.
    pub fn quill_ref(&self) -> String {
        let version = self
            .quill
            .metadata
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");
        format!("{}@{}", self.quill.name, version)
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
