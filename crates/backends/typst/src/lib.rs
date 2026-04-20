//! # Typst Backend for Quillmark
//!
//! This crate provides a complete Typst backend implementation that converts Markdown
//! documents to PDF and SVG formats via the Typst typesetting system.
//!
//! ## Overview
//!
//! The primary entry point is the [`TypstBackend`] struct, which implements the
//! [`Backend`] trait from `quillmark-core`. Users typically interact with this backend
//! through the high-level `Workflow` API from the `quillmark` crate.
//!
//! ## Features
//!
//! - Converts CommonMark Markdown to Typst markup
//! - Compiles Typst documents to PDF and SVG formats
//! - Provides template filters for YAML data transformation
//! - Manages fonts, assets, and packages dynamically
//! - Thread-safe for concurrent rendering
//!
//! ## Example Usage
//!
//! ```no_run
//! use quillmark_typst::TypstBackend;
//! use quillmark::{Backend, Quillmark, OutputFormat};
//!
//! let backend = TypstBackend::default();
//! let engine = Quillmark::new();
//! let quill = engine.quill_from_path("path/to/quill").unwrap();
//!
//! // Use with Workflow API (recommended)
//! // let workflow = Workflow::new(Box::new(backend), quill);
//! ```
//! ## Modules
//!
//! - [`convert`] - Markdown to Typst conversion utilities
//! - [`compile`] - Typst to PDF/SVG compilation functions
//!
//! Note: The `error_mapping` module provides internal utilities for converting Typst
//! diagnostics to Quillmark diagnostics and is not part of the public API.

pub mod compile;
pub mod convert;
mod error_mapping;

pub mod helper;
mod world;

/// Utilities exposed for fuzzing tests.
/// Not intended for general use.
#[doc(hidden)]
pub mod fuzz_utils {
    pub use super::helper::inject_json;
}

use convert::mark_to_typst;
use quillmark_core::{
    Artifact, Backend, CompiledDocument, Diagnostic, OutputFormat, Quill, QuillValue, RenderError,
    RenderOptions, RenderResult, Severity,
};
use std::collections::HashMap;

/// Typst backend implementation for Quillmark.
#[derive(Debug)]
pub struct TypstBackend;

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        &[OutputFormat::Pdf, OutputFormat::Svg, OutputFormat::Png]
    }

    fn plate_extension_types(&self) -> &'static [&'static str] {
        &[".typ"]
    }

    fn compile(
        &self,
        plate_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
        json_data: &serde_json::Value,
    ) -> Result<RenderResult, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        // Check if format is supported
        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!("{:?} not supported by {} backend", format, self.id()),
                    )
                    .with_code("backend::format_not_supported".to_string())
                    .with_hint(format!("Supported formats: {:?}", self.supported_formats())),
                ),
            });
        }

        // Serialize JSON value to string for injection into Typst
        let json_str = serde_json::to_string(json_data).unwrap_or_else(|_| "{}".to_string());

        match format {
            OutputFormat::Pdf => {
                let bytes = compile::compile_to_pdf(quill, plate_content, &json_str)?;
                let artifacts = vec![Artifact {
                    bytes,
                    output_format: OutputFormat::Pdf,
                }];
                Ok(RenderResult::new(artifacts, OutputFormat::Pdf))
            }
            OutputFormat::Svg => {
                let svg_pages = compile::compile_to_svg(quill, plate_content, &json_str)?;
                let artifacts = svg_pages
                    .into_iter()
                    .map(|bytes| Artifact {
                        bytes,
                        output_format: OutputFormat::Svg,
                    })
                    .collect();
                Ok(RenderResult::new(artifacts, OutputFormat::Svg))
            }
            OutputFormat::Png => {
                let png_pages = compile::compile_to_png(quill, plate_content, &json_str, opts.ppi)?;
                let artifacts = png_pages
                    .into_iter()
                    .map(|bytes| Artifact {
                        bytes,
                        output_format: OutputFormat::Png,
                    })
                    .collect();
                Ok(RenderResult::new(artifacts, OutputFormat::Png))
            }
            OutputFormat::Txt => Err(RenderError::FormatNotSupported {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!("Text output not supported by {} backend", self.id()),
                    )
                    .with_code("backend::format_not_supported".to_string())
                    .with_hint(format!("Supported formats: {:?}", self.supported_formats())),
                ),
            }),
        }
    }

    fn compile_to_document(
        &self,
        plate_content: &str,
        quill: &Quill,
        json_data: &serde_json::Value,
    ) -> Result<CompiledDocument, RenderError> {
        let json_str = serde_json::to_string(json_data).unwrap_or_else(|_| "{}".to_string());
        let document = compile::compile_to_document(quill, plate_content, &json_str)?;
        let page_count = document.pages.len();
        Ok(CompiledDocument::new(Box::new(document), page_count))
    }

    fn render_pages(
        &self,
        doc: &CompiledDocument,
        pages: Option<&[usize]>,
        format: OutputFormat,
        ppi: Option<f32>,
    ) -> Result<RenderResult, RenderError> {
        let paged_doc = doc
            .as_any()
            .downcast_ref::<typst::layout::PagedDocument>()
            .ok_or_else(|| RenderError::CompilationFailed {
                diags: vec![Diagnostic::new(
                    Severity::Error,
                    "Compiled document type mismatch for typst backend".to_string(),
                )
                .with_code("typst::compiled_document_type_mismatch".to_string())],
            })?;

        compile::render_document_pages(paged_doc, pages, format, ppi)
    }

    fn transform_fields(
        &self,
        fields: &HashMap<String, QuillValue>,
        schema: &QuillValue,
    ) -> HashMap<String, QuillValue> {
        transform_markdown_fields(fields, schema)
    }
}

impl Default for TypstBackend {
    /// Creates a new [`TypstBackend`] instance.
    fn default() -> Self {
        Self
    }
}

/// Check if a field schema indicates markdown content.
///
/// A field is considered markdown if it has:
/// - `contentMediaType = "text/markdown"`
fn is_markdown_field(field_schema: &serde_json::Value) -> bool {
    field_schema
        .get("contentMediaType")
        .and_then(|v| v.as_str())
        .map(|s| s == "text/markdown")
        .unwrap_or(false)
}

/// Check if a field schema indicates a date field.
///
/// A field is considered a date if it has:
/// - `type = "string"`
/// - `format = "date"`
fn is_date_field(field_schema: &serde_json::Value) -> bool {
    let is_string = field_schema
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s == "string")
        .unwrap_or(false);

    let is_date_format = field_schema
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s == "date")
        .unwrap_or(false);

    is_string && is_date_format
}

/// Transform markdown fields to Typst markup based on schema.
///
/// Identifies fields with `contentMediaType = "text/markdown"` and converts
/// their content using `mark_to_typst()`. This includes recursive handling
/// of CARDS arrays.
///
/// Also injects a `__meta__` key into the result containing the names of
/// converted fields, which the quillmark-helper package uses to auto-evaluate
/// markup strings into Typst content objects.
fn transform_markdown_fields(
    fields: &HashMap<String, QuillValue>,
    schema: &QuillValue,
) -> HashMap<String, QuillValue> {
    let mut result = fields.clone();
    let schema_json = schema.as_json();

    // Get the properties object from the schema
    let properties_obj = match schema_json.get("properties").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return result,
    };

    // Transform each field based on schema, collecting converted field names
    let mut content_field_names: Vec<&str> = Vec::new();
    for (field_name, field_value) in fields {
        if let Some(field_schema) = properties_obj.get(field_name) {
            if is_markdown_field(field_schema) {
                if let Some(content) = field_value.as_str() {
                    if let Ok(typst_markup) = mark_to_typst(content) {
                        result.insert(
                            field_name.clone(),
                            QuillValue::from_json(serde_json::json!(typst_markup)),
                        );
                        content_field_names.push(field_name);
                    }
                }
            }
        }
    }

    let date_fields: Vec<&str> = properties_obj
        .iter()
        .filter(|(_, fs)| is_date_field(fs))
        .map(|(name, _)| name.as_str())
        .collect();

    // Handle CARDS array recursively
    if let Some(cards_value) = result.get("CARDS") {
        if let Some(cards_array) = cards_value.as_array() {
            let transformed_cards = transform_cards_array(schema, cards_array);
            result.insert(
                "CARDS".to_string(),
                QuillValue::from_json(serde_json::Value::Array(transformed_cards)),
            );
        }
    }

    // Collect per-card-type content field names from schema $defs
    let mut card_content_fields = serde_json::Map::new();
    let mut card_date_fields = serde_json::Map::new();
    if let Some(defs) = schema_json.get("$defs").and_then(|v| v.as_object()) {
        for (def_name, def_schema) in defs {
            if let Some(card_type) = def_name.strip_suffix("_card") {
                let card_fields: Vec<&str> = def_schema
                    .get("properties")
                    .and_then(|v| v.as_object())
                    .map(|props| {
                        props
                            .iter()
                            .filter(|(_, fs)| is_markdown_field(fs))
                            .map(|(name, _)| name.as_str())
                            .collect()
                    })
                    .unwrap_or_default();
                if !card_fields.is_empty() {
                    card_content_fields.insert(
                        card_type.to_string(),
                        serde_json::Value::Array(
                            card_fields
                                .into_iter()
                                .map(|s| serde_json::Value::String(s.to_string()))
                                .collect(),
                        ),
                    );
                }

                let date_fields: Vec<&str> = def_schema
                    .get("properties")
                    .and_then(|v| v.as_object())
                    .map(|props| {
                        props
                            .iter()
                            .filter(|(_, fs)| is_date_field(fs))
                            .map(|(name, _)| name.as_str())
                            .collect()
                    })
                    .unwrap_or_default();
                if !date_fields.is_empty() {
                    card_date_fields.insert(
                        card_type.to_string(),
                        serde_json::Value::Array(
                            date_fields
                                .into_iter()
                                .map(|s| serde_json::Value::String(s.to_string()))
                                .collect(),
                        ),
                    );
                }
            }
        }
    }

    // Inject __meta__ so the helper package can auto-eval content fields
    result.insert(
        "__meta__".to_string(),
        QuillValue::from_json(serde_json::json!({
            "content_fields": content_field_names,
            "card_content_fields": card_content_fields,
            "date_fields": date_fields,
            "card_date_fields": card_date_fields,
        })),
    );

    result
}

/// Transform markdown fields in CARDS array items.
fn transform_cards_array(
    document_schema: &QuillValue,
    cards_array: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut transformed_cards = Vec::new();

    // Get definitions for card schemas
    let defs = document_schema
        .as_json()
        .get("$defs")
        .and_then(|v| v.as_object());

    for card in cards_array {
        if let Some(card_obj) = card.as_object() {
            if let Some(card_type) = card_obj.get("CARD").and_then(|v| v.as_str()) {
                // Construct the definition name: {type}_card
                let def_name = format!("{}_card", card_type);

                // Look up the schema for this card type
                if let Some(card_schema_json) = defs.and_then(|d| d.get(&def_name)) {
                    // Convert the card object to HashMap<String, QuillValue>
                    let mut card_fields: HashMap<String, QuillValue> = HashMap::new();
                    for (k, v) in card_obj {
                        card_fields.insert(k.clone(), QuillValue::from_json(v.clone()));
                    }

                    // Recursively transform this card's fields
                    let transformed_card_fields = transform_markdown_fields(
                        &card_fields,
                        &QuillValue::from_json(card_schema_json.clone()),
                    );

                    // Convert back to JSON Value
                    let mut transformed_card_obj = serde_json::Map::new();
                    for (k, v) in transformed_card_fields {
                        transformed_card_obj.insert(k, v.into_json());
                    }

                    transformed_cards.push(serde_json::Value::Object(transformed_card_obj));
                    continue;
                }
            }
        }

        // If not an object, no CARD type, or no matching schema, keep as-is
        transformed_cards.push(card.clone());
    }

    transformed_cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_backend_info() {
        let backend = TypstBackend;
        assert_eq!(backend.id(), "typst");
        assert!(backend.supported_formats().contains(&OutputFormat::Pdf));
        assert!(backend.supported_formats().contains(&OutputFormat::Svg));
    }

    #[test]
    fn test_is_markdown_field() {
        let markdown_schema = json!({
            "type": "string",
            "contentMediaType": "text/markdown"
        });
        assert!(is_markdown_field(&markdown_schema));

        let string_schema = json!({
            "type": "string"
        });
        assert!(!is_markdown_field(&string_schema));

        let other_media_type = json!({
            "type": "string",
            "contentMediaType": "text/plain"
        });
        assert!(!is_markdown_field(&other_media_type));
    }

    #[test]
    fn test_is_date_field() {
        let date_schema = json!({
            "type": "string",
            "format": "date"
        });
        assert!(is_date_field(&date_schema));

        let date_time_schema = json!({
            "type": "string",
            "format": "date-time"
        });
        assert!(!is_date_field(&date_time_schema));

        let non_string_date_schema = json!({
            "type": "number",
            "format": "date"
        });
        assert!(!is_date_field(&non_string_date_schema));
    }

    #[test]
    fn test_transform_markdown_fields_basic() {
        let schema = QuillValue::from_json(json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "BODY": { "type": "string", "contentMediaType": "text/markdown" }
            }
        }));

        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(json!("My Title")),
        );
        fields.insert(
            "BODY".to_string(),
            QuillValue::from_json(json!("This is **bold** text.")),
        );

        let result = transform_markdown_fields(&fields, &schema);

        // title should be unchanged
        assert_eq!(result.get("title").unwrap().as_str(), Some("My Title"));

        // BODY should be converted to Typst markup
        let body = result.get("BODY").unwrap().as_str().unwrap();
        assert!(body.contains("#strong[bold]"));
    }

    #[test]
    fn test_transform_markdown_fields_no_markdown() {
        let schema = QuillValue::from_json(json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "count": { "type": "number" }
            }
        }));

        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(json!("My Title")),
        );
        fields.insert("count".to_string(), QuillValue::from_json(json!(42)));

        let result = transform_markdown_fields(&fields, &schema);

        // All fields should be unchanged
        assert_eq!(result.get("title").unwrap().as_str(), Some("My Title"));
        assert_eq!(result.get("count").unwrap().as_i64(), Some(42));
    }

    #[test]
    fn test_transform_fields_trait_method() {
        let backend = TypstBackend;
        let schema = QuillValue::from_json(json!({
            "type": "object",
            "properties": {
                "BODY": { "type": "string", "contentMediaType": "text/markdown" }
            }
        }));

        let mut fields = HashMap::new();
        fields.insert(
            "BODY".to_string(),
            QuillValue::from_json(json!("_italic_ text")),
        );

        let result = backend.transform_fields(&fields, &schema);

        let body = result.get("BODY").unwrap().as_str().unwrap();
        assert!(body.contains("#emph[italic]"));
    }

    #[test]
    fn test_transform_markdown_fields_collects_top_level_date_metadata() {
        let schema = QuillValue::from_json(json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "date": { "type": "string", "format": "date" },
                "timestamp": { "type": "string", "format": "date-time" }
            }
        }));

        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(json!("My Title")),
        );

        let result = transform_markdown_fields(&fields, &schema);
        let meta = result.get("__meta__").expect("missing __meta__").as_json();

        assert_eq!(meta["date_fields"], json!(["date"]));
    }

    #[test]
    fn test_transform_markdown_fields_collects_card_date_metadata() {
        let schema = QuillValue::from_json(json!({
            "type": "object",
            "properties": {},
            "$defs": {
                "indorsement_card": {
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "format": "date" },
                        "created_at": { "type": "string", "format": "date-time" },
                        "BODY": { "type": "string", "contentMediaType": "text/markdown" }
                    }
                }
            }
        }));

        let fields = HashMap::new();
        let result = transform_markdown_fields(&fields, &schema);
        let meta = result.get("__meta__").expect("missing __meta__").as_json();

        assert_eq!(meta["card_date_fields"]["indorsement"], json!(["date"]));
    }
}
