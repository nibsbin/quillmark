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
//! use quillmark_core::{Backend, Quill, OutputFormat};
//!
//! let backend = TypstBackend::default();
//! let quill = Quill::from_path("path/to/quill").unwrap();
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

/// Embedded default Quill files
mod embedded {
    pub const QUILL_YAML: &str = include_str!("../default_quill/Quill.yaml");
    pub const PLATE_TYP: &str = include_str!("../default_quill/plate.typ");
    pub const EXAMPLE_MD: &str = include_str!("../default_quill/example.md");
}

/// Utilities exposed for fuzzing tests.
/// Not intended for general use.
#[doc(hidden)]
pub mod fuzz_utils {
    pub use super::helper::inject_json;
}

use convert::mark_to_typst;
use quillmark_core::{
    Artifact, Backend, Diagnostic, OutputFormat, Quill, QuillValue, RenderError, RenderOptions,
    RenderResult, Severity,
};
use std::collections::HashMap;

/// Typst backend implementation for Quillmark.
pub struct TypstBackend;

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        &[OutputFormat::Pdf, OutputFormat::Svg]
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

    fn transform_fields(
        &self,
        fields: &HashMap<String, QuillValue>,
        schema: &QuillValue,
    ) -> HashMap<String, QuillValue> {
        transform_markdown_fields(fields, schema)
    }

    fn default_quill(&self) -> Option<Quill> {
        use quillmark_core::FileTreeNode;

        // Build file tree from embedded files
        let mut files = HashMap::new();
        files.insert(
            "Quill.yaml".to_string(),
            FileTreeNode::File {
                contents: embedded::QUILL_YAML.as_bytes().to_vec(),
            },
        );
        files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: embedded::PLATE_TYP.as_bytes().to_vec(),
            },
        );
        files.insert(
            "example.md".to_string(),
            FileTreeNode::File {
                contents: embedded::EXAMPLE_MD.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files };

        // Try to create Quill from tree, return None if it fails
        Quill::from_tree(root).ok()
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

/// Transform markdown fields to Typst markup based on schema.
///
/// Identifies fields with `contentMediaType = "text/markdown"` and converts
/// their content using `mark_to_typst()`. This includes recursive handling
/// of CARDS arrays.
fn transform_markdown_fields(
    fields: &HashMap<String, QuillValue>,
    schema: &QuillValue,
) -> HashMap<String, QuillValue> {
    let mut result = fields.clone();

    // Get the properties object from the schema
    let properties = match schema.as_json().get("properties") {
        Some(props) => props,
        None => return result,
    };

    let properties_obj = match properties.as_object() {
        Some(obj) => obj,
        None => return result,
    };

    // Transform each field based on schema
    for (field_name, field_value) in fields {
        if let Some(field_schema) = properties_obj.get(field_name) {
            // Check if this is a markdown field
            if is_markdown_field(field_schema) {
                if let Some(content) = field_value.as_str() {
                    // Convert markdown to Typst markup
                    if let Ok(typst_markup) = mark_to_typst(content) {
                        result.insert(
                            field_name.clone(),
                            QuillValue::from_json(serde_json::json!(typst_markup)),
                        );
                    }
                }
            }
        }
    }

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
    fn test_default_quill_schema_has_body() {
        let backend = TypstBackend;
        let quill = backend
            .default_quill()
            .expect("Failed to load default quill");

        // Inspect the schema derived from Quill.yaml
        let schema_json = quill.schema.as_json();
        let properties = schema_json
            .get("properties")
            .expect("Schema missing properties");
        let body_schema = properties.get("BODY").expect("Schema missing BODY field");

        assert_eq!(
            body_schema.get("contentMediaType").and_then(|v| v.as_str()),
            Some("text/markdown"),
            "BODY field should be marked as text/markdown"
        );
    }
}
