use indexmap::IndexMap;
use quillmark_core::{
    normalize::normalize_document, Backend, Card, Diagnostic, Document, OutputFormat, Quill,
    QuillValue, RenderError, RenderOptions, RenderResult, RenderSession, Severity,
};
use std::sync::Arc;

/// Sealed workflow for rendering Markdown documents. See [module docs](super) for usage patterns.
pub struct Workflow {
    backend: Arc<dyn Backend>,
    quill: Quill,
}

struct PreparedRenderContext {
    json_data: serde_json::Value,
    plate_content: String,
    prepared_quill: Quill,
}

impl Workflow {
    /// Create a new Workflow with the specified backend and quill.
    pub fn new(backend: Arc<dyn Backend>, quill: Quill) -> Result<Self, RenderError> {
        // Quills are validated at construction time before workflow creation.
        Ok(Self { backend, quill })
    }

    /// Compile a Document to JSON data suitable for the backend.
    ///
    /// Applies coercion, validation, normalization, and schema defaults, then
    /// calls [`Document::to_plate_json`] to produce the wire format.
    pub fn compile_data(&self, doc: &Document) -> Result<serde_json::Value, RenderError> {
        // Coerce frontmatter fields against the schema.
        let coerced_frontmatter = self
            .quill
            .config
            .coerce_frontmatter(doc.frontmatter())
            .map_err(|e| RenderError::ValidationFailed {
                diag: Box::new(
                    Diagnostic::new(Severity::Error, e.to_string())
                        .with_code("validation::coercion_failed".to_string())
                        .with_hint(
                            "Ensure all fields can be coerced to their declared types".to_string(),
                        ),
                ),
            })?;

        // Coerce card fields against per-card schemas.
        let mut coerced_cards: Vec<Card> = Vec::new();
        for card in doc.cards() {
            let coerced_fields = self
                .quill
                .config
                .coerce_card(card.tag(), card.fields())
                .map_err(|e| RenderError::ValidationFailed {
                    diag: Box::new(
                        Diagnostic::new(Severity::Error, e.to_string())
                            .with_code("validation::coercion_failed".to_string())
                            .with_hint(
                                "Ensure all card fields can be coerced to their declared types"
                                    .to_string(),
                            ),
                    ),
                })?;
            coerced_cards.push(Card::new_internal(
                card.tag().to_string(),
                coerced_fields,
                card.body().to_string(),
            ));
        }

        let coerced_doc = Document::new_internal(
            doc.quill_reference().clone(),
            coerced_frontmatter,
            doc.body().to_string(),
            coerced_cards,
            doc.warnings().to_vec(),
        );

        self.validate_document(&coerced_doc)?;

        // Normalize: strip bidi + fix HTML comment fences in body regions.
        let normalized = normalize_document(coerced_doc)?;

        // Apply schema defaults to frontmatter.
        let frontmatter_with_defaults = self.apply_frontmatter_defaults(normalized.frontmatter());

        // Apply per-card defaults.
        let cards_with_defaults: Vec<Card> = normalized
            .cards()
            .iter()
            .map(|card| {
                let fields_with_defaults = self.apply_card_defaults(card.tag(), card.fields());
                Card::new_internal(
                    card.tag().to_string(),
                    fields_with_defaults,
                    card.body().to_string(),
                )
            })
            .collect();

        // Rebuild document with defaults applied.
        let final_doc = Document::new_internal(
            normalized.quill_reference().clone(),
            frontmatter_with_defaults,
            normalized.body().to_string(),
            cards_with_defaults,
            normalized.warnings().to_vec(),
        );

        // Build the plate wire format.
        Ok(final_doc.to_plate_json())
    }

    pub fn render(
        &self,
        doc: &Document,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        self.render_with_options(doc, format, None)
    }

    /// Open a backend-specific iterative render session.
    pub fn open(&self, doc: &Document) -> Result<RenderSession, RenderError> {
        let context = self.prepare_render_context(doc)?;
        self.backend.open(
            &context.plate_content,
            &context.prepared_quill,
            &context.json_data,
        )
    }

    /// Render with explicit pixels-per-inch for raster formats (PNG).
    ///
    /// `ppi` is ignored for vector/document formats (PDF, SVG, TXT).
    /// When `None`, defaults to 144.0 (2x at 72pt/inch).
    pub fn render_with_options(
        &self,
        doc: &Document,
        format: Option<OutputFormat>,
        ppi: Option<f32>,
    ) -> Result<RenderResult, RenderError> {
        let context = self.prepare_render_context(doc)?;
        self.render_plate_with_quill_and_data(
            &context.plate_content,
            format,
            ppi,
            &context.prepared_quill,
            &context.json_data,
        )
    }

    fn prepare_render_context(&self, doc: &Document) -> Result<PreparedRenderContext, RenderError> {
        Ok(PreparedRenderContext {
            json_data: self.compile_data(doc)?,
            plate_content: self.get_plate_content()?.unwrap_or_default(),
            prepared_quill: self.quill.clone(),
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
            pages: None,
        };

        self.backend
            .open(content, quill, json_data)?
            .render(&render_opts)
    }

    /// Apply frontmatter defaults from QuillConfig.
    fn apply_frontmatter_defaults(
        &self,
        frontmatter: &IndexMap<String, QuillValue>,
    ) -> IndexMap<String, QuillValue> {
        let mut result = frontmatter.clone();
        for (field_name, default_value) in self.quill.config.defaults() {
            if !result.contains_key(&field_name) {
                result.insert(field_name, default_value);
            }
        }
        result
    }

    /// Apply per-card defaults from QuillConfig.
    fn apply_card_defaults(
        &self,
        card_tag: &str,
        fields: &IndexMap<String, QuillValue>,
    ) -> IndexMap<String, QuillValue> {
        let mut result = fields.clone();
        if let Some(card_defaults) = self.quill.config.card_defaults(card_tag) {
            for (field_name, default_value) in card_defaults {
                if !result.contains_key(&field_name) {
                    result.insert(field_name, default_value);
                }
            }
        }
        result
    }

    /// Get the plate content directly from the quill.
    fn get_plate_content(&self) -> Result<Option<String>, RenderError> {
        match &self.quill.plate {
            Some(s) if !s.is_empty() => Ok(Some(s.clone())),
            _ => Ok(None),
        }
    }

    /// Perform a dry run validation without backend compilation.
    pub fn dry_run(&self, doc: &Document) -> Result<(), RenderError> {
        let coerced_frontmatter = self
            .quill
            .config
            .coerce_frontmatter(doc.frontmatter())
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
        let mut coerced_cards: Vec<Card> = Vec::new();
        for card in doc.cards() {
            let coerced_fields = self
                .quill
                .config
                .coerce_card(card.tag(), card.fields())
                .map_err(|e| RenderError::ValidationFailed {
                    diag: Box::new(
                        Diagnostic::new(Severity::Error, e.to_string())
                            .with_code("validation::coercion_failed".to_string())
                            .with_hint(
                                "Ensure all card fields can be coerced to their declared types"
                                    .to_string(),
                            ),
                    ),
                })?;
            coerced_cards.push(Card::new_internal(
                card.tag().to_string(),
                coerced_fields,
                card.body().to_string(),
            ));
        }
        let coerced_doc = Document::new_internal(
            doc.quill_reference().clone(),
            coerced_frontmatter,
            doc.body().to_string(),
            coerced_cards,
            doc.warnings().to_vec(),
        );
        self.validate_document(&coerced_doc)?;
        Ok(())
    }

    /// Validate a Document against the Quill's schema.
    pub fn validate_schema(&self, doc: &Document) -> Result<(), RenderError> {
        self.validate_document(doc)
    }

    /// Internal validation method.
    fn validate_document(&self, doc: &Document) -> Result<(), RenderError> {
        match self.quill.config.validate_document(doc) {
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

}
