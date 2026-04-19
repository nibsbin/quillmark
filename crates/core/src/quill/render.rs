use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    normalize::normalize_document,
    quill::{FieldSchema, FieldType},
    Backend, CompiledDocument, Diagnostic, ParsedDocument, Quill, QuillValue, RenderError,
    RenderOptions, RenderResult, Severity,
};

/// Input to a render or compile operation — either raw Markdown or a pre-parsed document.
pub enum QuillInput {
    /// Raw Markdown source (will be parsed internally)
    Markdown(String),
    /// Pre-parsed document
    Parsed(ParsedDocument),
}

impl From<String> for QuillInput {
    fn from(s: String) -> Self {
        QuillInput::Markdown(s)
    }
}

impl From<&str> for QuillInput {
    fn from(s: &str) -> Self {
        QuillInput::Markdown(s.to_string())
    }
}

impl From<ParsedDocument> for QuillInput {
    fn from(p: ParsedDocument) -> Self {
        QuillInput::Parsed(p)
    }
}

impl Quill {
    /// Attach a backend to this quill, returning a render-ready quill.
    pub fn with_backend(mut self, backend: Arc<dyn Backend>) -> Self {
        self.resolved_backend = Some(backend);
        self
    }

    /// Return the resolved backend, if one has been attached.
    pub fn backend(&self) -> Option<&Arc<dyn Backend>> {
        self.resolved_backend.as_ref()
    }

    /// Render a document to final artifacts.
    pub fn render(
        &self,
        input: impl Into<QuillInput>,
        opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        let backend = self.require_backend()?;
        let (parsed, ref_mismatch_warning) = self.resolve_input(input.into())?;
        let json_data = self.compile_data_internal(&parsed, backend)?;
        let plate_content = self.plate.clone().unwrap_or_default();
        let mut result = backend.compile(&plate_content, self, opts, &json_data)?;
        if let Some(w) = ref_mismatch_warning {
            result.warnings.push(w);
        }
        Ok(result)
    }

    /// Compile a document to a backend-specific compiled handle for page-selective rendering.
    pub fn compile(&self, input: impl Into<QuillInput>) -> Result<CompiledDocument, RenderError> {
        let backend = self.require_backend()?;
        let (parsed, _) = self.resolve_input(input.into())?;
        let json_data = self.compile_data_internal(&parsed, backend)?;
        let plate_content = self.plate.clone().unwrap_or_default();
        backend.compile_to_document(&plate_content, self, &json_data)
    }

    fn require_backend(&self) -> Result<&Arc<dyn Backend>, RenderError> {
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

    fn resolve_input(
        &self,
        input: QuillInput,
    ) -> Result<(ParsedDocument, Option<Diagnostic>), RenderError> {
        match input {
            QuillInput::Markdown(md) => {
                let parsed = ParsedDocument::from_markdown(&md)?;
                let warning = self.ref_mismatch_warning(&parsed);
                Ok((parsed, warning))
            }
            QuillInput::Parsed(parsed) => {
                let warning = self.ref_mismatch_warning(&parsed);
                Ok((parsed, warning))
            }
        }
    }

    fn ref_mismatch_warning(&self, parsed: &ParsedDocument) -> Option<Diagnostic> {
        let doc_ref = parsed.quill_reference().name.as_str();
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
        parsed: &ParsedDocument,
        backend: &Arc<dyn Backend>,
    ) -> Result<serde_json::Value, RenderError> {
        let coerced_fields = self
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
        self.validate_fields(&parsed_coerced)?;

        let normalized = normalize_document(parsed_coerced)?;

        let transform_schema = self.build_transform_schema();
        let transformed_fields = backend.transform_fields(normalized.fields(), &transform_schema);

        let fields_with_defaults = self.apply_schema_defaults(&transformed_fields);
        Ok(Self::fields_to_json(&fields_with_defaults))
    }

    fn validate_fields(&self, parsed: &ParsedDocument) -> Result<(), RenderError> {
        match self.config.validate(parsed.fields()) {
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

    fn apply_schema_defaults(
        &self,
        fields: &HashMap<String, QuillValue>,
    ) -> HashMap<String, QuillValue> {
        let mut result = fields.clone();
        for (field_name, default_value) in self.config.defaults() {
            if !result.contains_key(&field_name) {
                result.insert(field_name, default_value);
            }
        }
        result
    }

    fn fields_to_json(fields: &HashMap<String, QuillValue>) -> serde_json::Value {
        let mut json_map = serde_json::Map::new();
        for (key, value) in fields {
            json_map.insert(key.clone(), value.as_json().clone());
        }
        serde_json::Value::Object(json_map)
    }

    pub(crate) fn build_transform_schema(&self) -> QuillValue {
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

    /// Render to the first supported output format (convenience helper).
    pub fn render_default(
        &self,
        input: impl Into<QuillInput>,
    ) -> Result<RenderResult, RenderError> {
        let backend = self.require_backend()?;
        let output_format = backend.supported_formats().first().copied();
        self.render(
            input,
            &RenderOptions {
                output_format,
                ppi: None,
            },
        )
    }
}
