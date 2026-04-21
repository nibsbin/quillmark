use std::sync::Arc;

use indexmap::IndexMap;

use crate::{
    document::Card,
    normalize::normalize_document,
    quill::{FieldSchema, FieldType},
    Diagnostic, Document, Quill, QuillValue, RenderError, RenderOptions, RenderResult,
    Severity,
};

impl Quill {
    /// Attach a backend to this quill, returning a render-ready quill.
    pub fn with_backend(mut self, backend: Arc<dyn crate::Backend>) -> Self {
        self.resolved_backend = Some(backend);
        self
    }

    /// Return the resolved backend, if one has been attached.
    pub fn backend(&self) -> Option<&Arc<dyn crate::Backend>> {
        self.resolved_backend.as_ref()
    }

    /// Render a document to final artifacts.
    ///
    /// Note: page selection (`RenderOptions.pages`) is ignored in this one-shot
    /// convenience path. Use `open(...).render(...)` for page-selective rendering.
    pub fn render(
        &self,
        doc: Document,
        opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        let all_pages_opts = RenderOptions {
            output_format: opts.output_format,
            ppi: opts.ppi,
            pages: None,
        };
        self.open(doc)?.render(&all_pages_opts)
    }

    /// Open an iterative render session for this document.
    pub fn open(&self, doc: Document) -> Result<crate::RenderSession, RenderError> {
        let backend = self.require_backend()?;
        let warning = self.ref_mismatch_warning(&doc);
        let json_data = self.compile_data_internal(&doc)?;
        let plate_content = self.plate.clone().unwrap_or_default();
        let session = backend.open(&plate_content, self, &json_data)?;
        Ok(session.with_warning(warning))
    }

    fn require_backend(&self) -> Result<&Arc<dyn crate::Backend>, RenderError> {
        self.resolved_backend.as_ref().ok_or_else(|| RenderError::NoBackend {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Quill '{}' has no backend attached; use engine.quill() or engine.quill_from_path() instead",
                        self.name
                    ),
                )
                .with_code("quill::no_backend".to_string())
                .with_hint(
                    "Create render-ready quills via engine.quill(tree) or engine.quill_from_path(path)".to_string(),
                ),
            ),
        })
    }

    fn ref_mismatch_warning(&self, doc: &Document) -> Option<Diagnostic> {
        let doc_ref = doc.quill_reference().name.as_str();
        if doc_ref != self.name {
            Some(
                Diagnostic::new(
                    Severity::Warning,
                    format!(
                        "document declares QUILL '{}' but was rendered with '{}'",
                        doc_ref, self.name
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

    pub(crate) fn compile_data_internal(
        &self,
        doc: &Document,
    ) -> Result<serde_json::Value, RenderError> {
        // Coerce frontmatter
        let coerced_frontmatter = self
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

        // Coerce card fields
        let mut coerced_cards: Vec<Card> = Vec::new();
        for card in doc.cards() {
            let coerced_fields = self
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
            coerced_cards.push(Card::new(
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

        self.validate_fields(&coerced_doc)?;

        let normalized = normalize_document(coerced_doc)?;

        // Apply frontmatter defaults
        let frontmatter_with_defaults: IndexMap<String, QuillValue> = {
            let mut fm = normalized.frontmatter().clone();
            for (field_name, default_value) in self.config.defaults() {
                if !fm.contains_key(&field_name) {
                    fm.insert(field_name, default_value);
                }
            }
            fm
        };

        // Apply card defaults
        let cards_with_defaults: Vec<Card> = normalized
            .cards()
            .iter()
            .map(|card| {
                let mut cf = card.fields().clone();
                if let Some(card_defaults) = self.config.card_defaults(card.tag()) {
                    for (k, v) in card_defaults {
                        if !cf.contains_key(&k) {
                            cf.insert(k, v);
                        }
                    }
                }
                Card::new(card.tag().to_string(), cf, card.body().to_string())
            })
            .collect();

        let final_doc = Document::new_internal(
            normalized.quill_reference().clone(),
            frontmatter_with_defaults,
            normalized.body().to_string(),
            cards_with_defaults,
            normalized.warnings().to_vec(),
        );

        Ok(final_doc.to_plate_json())
    }

    fn validate_fields(&self, doc: &Document) -> Result<(), RenderError> {
        match self.config.validate_document(doc) {
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

    pub fn build_transform_schema(&self) -> QuillValue {
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
                        let mut props = serde_json::Map::new();
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
        for (name, field) in &self.config.main().fields {
            properties.insert(name.clone(), field_to_schema(field));
        }
        properties.insert(
            "BODY".to_string(),
            serde_json::json!({ "type": "string", "contentMediaType": "text/markdown" }),
        );

        let mut defs = serde_json::Map::new();
        for card in self.config.card_definitions() {
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
}
