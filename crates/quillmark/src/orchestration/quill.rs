//! Renderable `Quill` — the engine-constructed composition of a
//! [`QuillSource`] with a resolved backend.

use indexmap::IndexMap;
use std::sync::Arc;

use quillmark_core::{
    normalize::normalize_document, Backend, Card, Diagnostic, Document, OutputFormat, QuillSource,
    QuillValue, RenderError, RenderOptions, RenderResult, RenderSession, Severity,
};

/// Renderable quill. Composes an [`Arc<QuillSource>`] with a resolved
/// [`Backend`]. Constructed by the engine; immutable once created.
#[derive(Clone)]
pub struct Quill {
    source: Arc<QuillSource>,
    backend: Arc<dyn Backend>,
}

struct PreparedRenderContext {
    json_data: serde_json::Value,
    plate_content: String,
}

impl Quill {
    /// Construct a Quill from a source and a resolved backend.
    ///
    /// Engine-internal; external callers should use
    /// [`crate::Quillmark::quill`] or [`crate::Quillmark::quill_from_path`].
    pub(crate) fn new(source: Arc<QuillSource>, backend: Arc<dyn Backend>) -> Self {
        Self { source, backend }
    }

    /// The underlying quill source.
    pub fn source(&self) -> &QuillSource {
        &self.source
    }

    /// The resolved backend identifier (e.g. `"typst"`).
    pub fn backend_id(&self) -> &str {
        self.backend.id()
    }

    /// Supported output formats for this quill's backend.
    pub fn supported_formats(&self) -> &'static [OutputFormat] {
        self.backend.supported_formats()
    }

    /// The quill's declared name.
    pub fn name(&self) -> &str {
        &self.source.name
    }

    /// Render a document to final artifacts.
    pub fn render(
        &self,
        doc: &Document,
        format: Option<OutputFormat>,
    ) -> Result<RenderResult, RenderError> {
        self.render_with_options(doc, format, None)
    }

    /// Render with explicit pixels-per-inch for raster formats (PNG).
    ///
    /// `ppi` is ignored for vector/document formats (PDF, SVG, TXT).
    /// When `None`, the backend's default is used.
    pub fn render_with_options(
        &self,
        doc: &Document,
        format: Option<OutputFormat>,
        ppi: Option<f32>,
    ) -> Result<RenderResult, RenderError> {
        let context = self.prepare_render_context(doc)?;
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

        let warning = self.ref_mismatch_warning(doc);
        let session =
            self.backend
                .open(&context.plate_content, &self.source, &context.json_data)?;
        let session = session.with_warning(warning);
        session.render(&render_opts)
    }

    /// Open an iterative render session for this document.
    pub fn open(&self, doc: &Document) -> Result<RenderSession, RenderError> {
        let context = self.prepare_render_context(doc)?;
        let warning = self.ref_mismatch_warning(doc);
        let session =
            self.backend
                .open(&context.plate_content, &self.source, &context.json_data)?;
        Ok(session.with_warning(warning))
    }

    /// Compile a Document to JSON data suitable for the backend.
    ///
    /// Applies coercion, validation, normalization, and schema defaults, then
    /// calls [`Document::to_plate_json`] to produce the wire format.
    pub fn compile_data(&self, doc: &Document) -> Result<serde_json::Value, RenderError> {
        // Coerce frontmatter fields against the schema.
        let coerced_frontmatter = self
            .source
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
                .source
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

    fn prepare_render_context(&self, doc: &Document) -> Result<PreparedRenderContext, RenderError> {
        Ok(PreparedRenderContext {
            json_data: self.compile_data(doc)?,
            plate_content: self.plate_content().unwrap_or_default(),
        })
    }

    fn ref_mismatch_warning(&self, doc: &Document) -> Option<Diagnostic> {
        let doc_ref = doc.quill_reference().name.as_str();
        if doc_ref != self.source.name {
            Some(
                Diagnostic::new(
                    Severity::Warning,
                    format!(
                        "document declares QUILL '{}' but was rendered with '{}'",
                        doc_ref, self.source.name
                    ),
                )
                .with_code("quill::ref_mismatch".to_string())
                .with_hint(
                    "the QUILL field is informational; ensure you are rendering with the intended quill"
                        .to_string(),
                ),
            )
        } else {
            None
        }
    }

    fn apply_frontmatter_defaults(
        &self,
        frontmatter: &IndexMap<String, QuillValue>,
    ) -> IndexMap<String, QuillValue> {
        let mut result = frontmatter.clone();
        for (field_name, default_value) in self.source.config.defaults() {
            if !result.contains_key(&field_name) {
                result.insert(field_name, default_value);
            }
        }
        result
    }

    fn apply_card_defaults(
        &self,
        card_tag: &str,
        fields: &IndexMap<String, QuillValue>,
    ) -> IndexMap<String, QuillValue> {
        let mut result = fields.clone();
        if let Some(card_defaults) = self.source.config.card_defaults(card_tag) {
            for (field_name, default_value) in card_defaults {
                if !result.contains_key(&field_name) {
                    result.insert(field_name, default_value);
                }
            }
        }
        result
    }

    fn plate_content(&self) -> Option<String> {
        match &self.source.plate {
            Some(s) if !s.is_empty() => Some(s.clone()),
            _ => None,
        }
    }

    /// Perform a dry-run validation without backend compilation.
    pub fn dry_run(&self, doc: &Document) -> Result<(), RenderError> {
        let coerced_frontmatter = self
            .source
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
                .source
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

    fn validate_document(&self, doc: &Document) -> Result<(), RenderError> {
        match self.source.config.validate_document(doc) {
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
}

impl std::fmt::Debug for Quill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quill")
            .field("name", &self.source.name)
            .field("backend", &self.backend.id())
            .finish()
    }
}
