//! Renderable `Quill` — the engine-constructed composition of a
//! [`QuillSource`] with a resolved backend.

use indexmap::IndexMap;
use std::sync::Arc;

use quillmark_core::{
    normalize::normalize_document, Backend, Card, Diagnostic, Document, Frontmatter, OutputFormat,
    QuillSource, QuillValue, RenderError, RenderOptions, RenderResult, RenderSession, Sentinel,
    Severity,
};

use crate::form::{self, Form, FormCard};

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
        self.source.name()
    }

    /// Render a document to final artifacts.
    ///
    /// Pass `&RenderOptions::default()` for backend defaults (first supported
    /// format, backend-chosen ppi, all pages).
    pub fn render(
        &self,
        doc: &Document,
        opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        let session = self.open(doc)?;
        let resolved = self.resolve_options(opts);
        session.render(&resolved)
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

    fn resolve_options(&self, opts: &RenderOptions) -> RenderOptions {
        let output_format = opts
            .output_format
            .or_else(|| self.backend.supported_formats().first().copied());
        RenderOptions {
            output_format,
            ppi: opts.ppi,
            pages: opts.pages.clone(),
        }
    }

    /// Compile a Document to JSON data suitable for the backend.
    ///
    /// Applies coercion, validation, normalization, and schema defaults, then
    /// calls [`Document::to_plate_json`] to produce the wire format.
    pub fn compile_data(&self, doc: &Document) -> Result<serde_json::Value, RenderError> {
        // Coerce main-card frontmatter fields against the schema.
        let main_fields_map = doc.main().frontmatter().to_index_map();
        let coerced_frontmatter = self
            .source
            .config()
            .coerce_frontmatter(&main_fields_map)
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
            let card_fields_map = card.frontmatter().to_index_map();
            let coerced_fields = self
                .source
                .config()
                .coerce_card(&card.tag(), &card_fields_map)
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
            coerced_cards.push(Card::new_with_sentinel(
                Sentinel::Card(card.tag()),
                Frontmatter::from_index_map(coerced_fields),
                card.body().to_string(),
            ));
        }

        let coerced_main = Card::new_with_sentinel(
            Sentinel::Main(doc.quill_reference().clone()),
            Frontmatter::from_index_map(coerced_frontmatter),
            doc.main().body().to_string(),
        );
        let coerced_doc =
            Document::from_main_and_cards(coerced_main, coerced_cards, doc.warnings().to_vec());

        self.validate_document(&coerced_doc)?;

        // Normalize: strip bidi + fix HTML comment fences in body regions.
        let normalized = normalize_document(coerced_doc)?;

        // Apply schema defaults to frontmatter.
        let normalized_main_map = normalized.main().frontmatter().to_index_map();
        let frontmatter_with_defaults = self.apply_frontmatter_defaults(&normalized_main_map);

        // Apply per-card defaults.
        let cards_with_defaults: Vec<Card> = normalized
            .cards()
            .iter()
            .map(|card| {
                let card_map = card.frontmatter().to_index_map();
                let fields_with_defaults = self.apply_card_defaults(&card.tag(), &card_map);
                Card::new_with_sentinel(
                    Sentinel::Card(card.tag()),
                    Frontmatter::from_index_map(fields_with_defaults),
                    card.body().to_string(),
                )
            })
            .collect();

        // Rebuild document with defaults applied.
        let final_main = Card::new_with_sentinel(
            Sentinel::Main(normalized.quill_reference().clone()),
            Frontmatter::from_index_map(frontmatter_with_defaults),
            normalized.main().body().to_string(),
        );
        let final_doc = Document::from_main_and_cards(
            final_main,
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
        if doc_ref != self.source.name() {
            Some(
                Diagnostic::new(
                    Severity::Warning,
                    format!(
                        "document declares QUILL '{}' but was rendered with '{}'",
                        doc_ref,
                        self.source.name()
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
        for (field_name, default_value) in self.source.config().defaults() {
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
        if let Some(card_defaults) = self.source.config().card_type_defaults(card_tag) {
            for (field_name, default_value) in card_defaults {
                if !result.contains_key(&field_name) {
                    result.insert(field_name, default_value);
                }
            }
        }
        result
    }

    fn plate_content(&self) -> Option<String> {
        self.source
            .plate()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// The schema-aware form view of `doc` — the whole-document snapshot
    /// rendered through this quill's schema.
    ///
    /// For each schema-declared field on the main card and on every
    /// recognised card, the returned [`Form`] records the current value, the
    /// schema default, and a [`form::FormFieldSource`] label.
    ///
    /// **Snapshot semantics.** The result is a read-only snapshot — re-call
    /// after editing `doc`.
    ///
    /// **Unknown card tags** are dropped from [`Form::cards`] and surface as
    /// `form::unknown_card_tag` diagnostics. Validation errors are appended
    /// as `form::validation_error` diagnostics; the view itself is never
    /// altered or filtered by validation failures.
    pub fn form(&self, doc: &Document) -> Form {
        form::build_form(self, doc)
    }

    /// A blank form for the main card — no document values supplied. Every
    /// declared field's source is [`form::FormFieldSource::Default`] (when
    /// the schema declares a default) or [`form::FormFieldSource::Missing`].
    ///
    /// Useful as a starting state for a fresh document, or for previewing the
    /// main-card form without a document in hand.
    pub fn blank_main(&self) -> FormCard {
        FormCard::blank(&self.source.config().main)
    }

    /// A blank form for a card of the given type — no document values
    /// supplied. Returns `None` if `card_type` is not declared in the
    /// quill's schema.
    ///
    /// This is the "user is about to add a new card" view: the UI can render
    /// the form before the card is committed to the document.
    pub fn blank_card(&self, card_type: &str) -> Option<FormCard> {
        form::blank_card_for_tag(self, card_type)
    }

    /// Perform a dry-run validation without backend compilation.
    pub fn dry_run(&self, doc: &Document) -> Result<(), RenderError> {
        let main_fields_map = doc.main().frontmatter().to_index_map();
        let coerced_frontmatter = self
            .source
            .config()
            .coerce_frontmatter(&main_fields_map)
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
            let card_fields_map = card.frontmatter().to_index_map();
            let coerced_fields = self
                .source
                .config()
                .coerce_card(&card.tag(), &card_fields_map)
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
            coerced_cards.push(Card::new_with_sentinel(
                Sentinel::Card(card.tag()),
                Frontmatter::from_index_map(coerced_fields),
                card.body().to_string(),
            ));
        }
        let coerced_main = Card::new_with_sentinel(
            Sentinel::Main(doc.quill_reference().clone()),
            Frontmatter::from_index_map(coerced_frontmatter),
            doc.main().body().to_string(),
        );
        let coerced_doc =
            Document::from_main_and_cards(coerced_main, coerced_cards, doc.warnings().to_vec());
        self.validate_document(&coerced_doc)?;
        Ok(())
    }

    fn validate_document(&self, doc: &Document) -> Result<(), RenderError> {
        match self.source.config().validate_document(doc) {
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
            .field("name", &self.source.name())
            .field("backend", &self.backend.id())
            .finish()
    }
}
