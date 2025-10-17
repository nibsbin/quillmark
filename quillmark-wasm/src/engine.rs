//! Quillmark WASM Engine - Simplified API

use crate::error::QuillmarkError;
use crate::types::{
    FieldSchema, OutputFormat, ParsedDocument, QuillInfo, RenderOptions, RenderResult,
};
use serde::Serialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// Cross-platform helper to get current time in milliseconds as f64.
fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let dur = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        dur.as_millis() as f64
    }
}

/// Quillmark WASM Engine
///
/// Create once, register Quills, render markdown. That's it.
#[wasm_bindgen]
pub struct Quillmark {
    inner: quillmark::Quillmark,
    quills: HashMap<String, quillmark_core::Quill>,
}

#[wasm_bindgen]
impl Quillmark {
    /// JavaScript constructor: `new Quillmark()`
    #[wasm_bindgen(constructor)]
    pub fn new() -> Quillmark {
        Quillmark {
            inner: quillmark::Quillmark::new(),
            quills: HashMap::new(),
        }
    }

    /// Parse markdown into a ParsedDocument
    ///
    /// This is the first step in the workflow. The returned ParsedDocument contains
    /// the parsed YAML frontmatter fields and the quill_tag (if QUILL field is present).
    #[wasm_bindgen(js_name = parseMarkdown)]
    pub fn parse_markdown(markdown: &str) -> Result<JsValue, JsValue> {
        let parsed = quillmark_core::ParsedDocument::from_markdown(markdown).map_err(|e| {
            QuillmarkError::new(
                format!("Failed to parse markdown: {}", e),
                None,
                Some("Check markdown syntax and YAML frontmatter".to_string()),
            )
            .to_js_value()
        })?;

        // Convert to WASM type
        let quill_tag = parsed.quill_tag().map(|s| s.to_string());

        // Convert fields HashMap to JSON
        let mut fields_obj = serde_json::Map::new();
        for (key, value) in parsed.fields() {
            fields_obj.insert(key.clone(), value.as_json().clone());
        }
        let fields = serde_json::Value::Object(fields_obj);

        let wasm_parsed = ParsedDocument { fields, quill_tag };

        // Use a serializer that converts HashMaps to plain objects instead of ES6 Maps
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        wasm_parsed.serialize(&serializer).map_err(|e| {
            QuillmarkError::new(
                format!("Failed to serialize ParsedDocument: {}", e),
                None,
                None,
            )
            .to_js_value()
        })
    }

    /// Register a Quill template bundle
    ///
    /// Accepts either a JSON string or a JsValue object representing the Quill file tree.
    /// Validation happens automatically on registration.
    #[wasm_bindgen(js_name = registerQuill)]
    pub fn register_quill(&mut self, name: &str, quill_json: JsValue) -> Result<(), JsValue> {
        // Convert JsValue to JSON string
        let json_str = if quill_json.is_string() {
            quill_json.as_string().ok_or_else(|| {
                QuillmarkError::new(
                    "Failed to convert JsValue to string".to_string(),
                    None,
                    None,
                )
                .to_js_value()
            })?
        } else {
            js_sys::JSON::stringify(&quill_json)
                .map_err(|e| {
                    QuillmarkError::new(
                        format!("Failed to serialize Quill JSON: {:?}", e),
                        None,
                        Some("Ensure the Quill object has the correct structure".to_string()),
                    )
                    .to_js_value()
                })?
                .as_string()
                .ok_or_else(|| {
                    QuillmarkError::new("Failed to convert JSON to string".to_string(), None, None)
                        .to_js_value()
                })?
        };

        // Parse and validate Quill
        let quill = quillmark_core::Quill::from_json(&json_str).map_err(|e| {
            QuillmarkError::new(
                format!("Failed to parse Quill: {}", e),
                None,
                Some("Ensure Quill.toml exists and is valid TOML".to_string()),
            )
            .to_js_value()
        })?;

        // Validate
        quill.validate().map_err(|e| {
            QuillmarkError::new(format!("Quill validation failed: {}", e), None, None).to_js_value()
        })?;

        // Register
        self.inner.register_quill(quill.clone());
        self.quills.insert(name.to_string(), quill);

        Ok(())
    }

    /// Get shallow information about a registered Quill
    ///
    /// This returns metadata, backend info, field schemas, and supported formats
    /// that consumers need to configure render options for the next step.
    #[wasm_bindgen(js_name = getQuillInfo)]
    pub fn get_quill_info(&self, name: &str) -> Result<JsValue, JsValue> {
        let quill = self.quills.get(name).ok_or_else(|| {
            QuillmarkError::new(
                format!("Quill '{}' not registered", name),
                None,
                Some("Use registerQuill() before getting quill info".to_string()),
            )
            .to_js_value()
        })?;

        // Get backend ID
        let backend_id = &quill.backend;

        // Create workflow to get supported formats
        let workflow = self.inner.workflow_from_quill_name(name).map_err(|e| {
            QuillmarkError::new(
                format!("Failed to create workflow for quill '{}': {}", name, e),
                None,
                None,
            )
            .to_js_value()
        })?;

        let supported_formats: Vec<OutputFormat> = workflow
            .supported_formats()
            .iter()
            .map(|&f| f.into())
            .collect();

        // Convert metadata to JSON
        let mut metadata_json = std::collections::HashMap::new();
        for (key, value) in &quill.metadata {
            metadata_json.insert(key.clone(), value.as_json().clone());
        }

        // Convert field schemas
        let field_schemas: std::collections::HashMap<String, FieldSchema> = quill
            .field_schemas
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().into()))
            .collect();

        let quill_info = QuillInfo {
            name: quill.name.clone(),
            backend: backend_id.clone(),
            metadata: metadata_json,
            example: quill.example.clone(),
            field_schemas,
            supported_formats,
        };

        // Use a serializer that converts HashMaps to plain objects instead of ES6 Maps
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        quill_info.serialize(&serializer).map_err(|e| {
            QuillmarkError::new(format!("Failed to serialize QuillInfo: {}", e), None, None)
                .to_js_value()
        })
    }

    /// Process markdown through template engine (debugging)
    ///
    /// Returns template source code (Typst, LaTeX, etc.)
    #[wasm_bindgen(js_name = renderGlue)]
    pub fn render_glue(&mut self, quill_name: &str, markdown: &str) -> Result<String, JsValue> {
        // Parse markdown first
        let parsed = quillmark_core::ParsedDocument::from_markdown(markdown).map_err(|e| {
            QuillmarkError::new(
                format!("Failed to parse markdown: {}", e),
                None,
                Some("Check markdown syntax and YAML frontmatter".to_string()),
            )
            .to_js_value()
        })?;

        let workflow = self
            .inner
            .workflow_from_quill_name(quill_name)
            .map_err(|e| {
                QuillmarkError::new(
                    format!("Quill '{}' not found: {}", quill_name, e),
                    None,
                    Some("Use registerQuill() before rendering".to_string()),
                )
                .to_js_value()
            })?;

        workflow
            .process_glue_parsed(&parsed)
            .map_err(|e| QuillmarkError::from(e).to_js_value())
    }

    /// Render a ParsedDocument to final artifacts (PDF, SVG, TXT)
    ///
    /// Uses the Quill specified in options.quill_name if provided,
    /// otherwise infers it from the ParsedDocument's quill_tag field.
    #[wasm_bindgen]
    pub fn render(&mut self, parsed_doc: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
        // Parse the ParsedDocument from JsValue
        let parsed_wasm: ParsedDocument =
            serde_wasm_bindgen::from_value(parsed_doc).map_err(|e| {
                QuillmarkError::new(
                    format!("Invalid ParsedDocument: {}", e),
                    None,
                    Some("Ensure you pass a valid ParsedDocument from parseMarkdown()".to_string()),
                )
                .to_js_value()
            })?;

        // Reconstruct a core ParsedDocument from the WASM type
        // Convert JSON value to HashMap<String, QuillValue>
        let fields_json = parsed_wasm.fields;
        let mut fields = std::collections::HashMap::new();

        if let serde_json::Value::Object(obj) = fields_json {
            for (key, value) in obj {
                fields.insert(key, quillmark_core::value::QuillValue::from_json(value));
            }
        }

        let parsed =
            quillmark_core::ParsedDocument::with_quill_tag(fields, parsed_wasm.quill_tag.clone());

        let opts: RenderOptions = if options.is_undefined() || options.is_null() {
            RenderOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(|e| {
                QuillmarkError::new(
                    format!("Invalid render options: {}", e),
                    None,
                    Some("Check that format is 'pdf', 'svg', or 'txt'".to_string()),
                )
                .to_js_value()
            })?
        };

        // Determine which workflow to use
        let mut workflow = if let Some(quill_name) = opts.quill_name {
            // Use explicitly provided quill name (overrides quill_tag field)
            self.inner
                .workflow_from_quill_name(&quill_name)
                .map_err(|e| {
                    QuillmarkError::new(
                        format!("Quill '{}' not found: {}", quill_name, e),
                        None,
                        Some("Use registerQuill() before rendering".to_string()),
                    )
                    .to_js_value()
                })?
        } else if let Some(quill_tag) = parsed_wasm.quill_tag {
            // Use quill_tag from parsed document
            self.inner
                .workflow_from_quill_name(&quill_tag)
                .map_err(|e| {
                    QuillmarkError::new(
                        format!("Quill '{}' from QUILL field not found: {}", quill_tag, e),
                        None,
                        Some("Use registerQuill() before rendering".to_string()),
                    )
                    .to_js_value()
                })?
        } else {
            return Err(QuillmarkError::new(
                "No quill specified".to_string(),
                None,
                Some(
                    "Either add a 'QUILL: <name>' field in your markdown frontmatter or specify quillName in options"
                        .to_string(),
                ),
            )
            .to_js_value());
        };

        // Add assets if provided
        if let Some(assets) = opts.assets {
            for (filename, bytes) in assets {
                workflow.add_asset(filename, bytes).map_err(|e| {
                    QuillmarkError::new(format!("Failed to add asset: {}", e), None, None)
                        .to_js_value()
                })?;
            }
        }

        let start = now_ms();

        let output_format = opts.format.map(|f| f.into());
        let result = workflow
            .render(&parsed, output_format)
            .map_err(|e| QuillmarkError::from(e).to_js_value())?;

        let render_result = RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            render_time_ms: now_ms() - start,
        };

        // Use a serializer that converts HashMaps to plain objects instead of ES6 Maps
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        render_result.serialize(&serializer).map_err(|e| {
            QuillmarkError::new(format!("Failed to serialize result: {}", e), None, None)
                .to_js_value()
        })
    }

    /// List registered Quill names
    #[wasm_bindgen(js_name = listQuills)]
    pub fn list_quills(&self) -> Vec<String> {
        self.quills.keys().cloned().collect()
    }

    /// Unregister a Quill (free memory)
    #[wasm_bindgen(js_name = unregisterQuill)]
    pub fn unregister_quill(&mut self, name: &str) {
        self.quills.remove(name);
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests verify the serialization behavior but can only be fully
    // tested in a WASM environment. They are included here for documentation
    // and can be run with wasm-bindgen-test.

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_quill_info_serialization_uses_plain_objects() {
        use super::*;
        use crate::types::{FieldSchema, QuillInfo};
        use serde::Serialize;

        // Create a QuillInfo with HashMap fields
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("key1".to_string(), serde_json::json!("value1"));
        metadata.insert("key2".to_string(), serde_json::json!(42));

        let mut field_schemas = std::collections::HashMap::new();
        field_schemas.insert(
            "field1".to_string(),
            FieldSchema {
                r#type: Some("string".to_string()),
                required: true,
                description: "Test field".to_string(),
                example: None,
                default: None,
            },
        );

        let quill_info = QuillInfo {
            name: "test-quill".to_string(),
            backend: "typst".to_string(),
            metadata,
            example: None,
            field_schemas,
            supported_formats: vec![OutputFormat::Pdf],
        };

        // Serialize using json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = quill_info
            .serialize(&serializer)
            .expect("serialization failed");

        // Convert to JSON string to verify structure
        let json_string = js_sys::JSON::stringify(&js_value)
            .expect("stringify failed")
            .as_string()
            .expect("as_string failed");

        // Verify that the JSON contains object-style fields (not Map)
        assert!(json_string.contains(r#""metadata""#));
        assert!(json_string.contains(r#""key1":"value1""#));
        assert!(json_string.contains(r#""fieldSchemas""#));
        assert!(json_string.contains(r#""field1""#));

        // Parse back to verify it's a valid JSON object structure
        let parsed: serde_json::Value =
            serde_json::from_str(&json_string).expect("JSON parse failed");

        // Verify metadata is an object (not an array which Map might serialize to)
        assert!(parsed["metadata"].is_object());
        assert_eq!(parsed["metadata"]["key1"], "value1");
        assert_eq!(parsed["metadata"]["key2"], 42);

        // Verify fieldSchemas is an object
        assert!(parsed["fieldSchemas"].is_object());
        assert!(parsed["fieldSchemas"]["field1"].is_object());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_parsed_document_serialization_uses_plain_objects() {
        use super::*;
        use crate::types::ParsedDocument;
        use serde::Serialize;

        // Create a ParsedDocument with a fields object
        let mut fields_map = serde_json::Map::new();
        fields_map.insert("title".to_string(), serde_json::json!("Test Document"));
        fields_map.insert("author".to_string(), serde_json::json!("Test Author"));
        let fields = serde_json::Value::Object(fields_map);

        let parsed_doc = ParsedDocument {
            fields,
            quill_tag: Some("test-quill".to_string()),
        };

        // Serialize using json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = parsed_doc
            .serialize(&serializer)
            .expect("serialization failed");

        // Convert to JSON string to verify structure
        let json_string = js_sys::JSON::stringify(&js_value)
            .expect("stringify failed")
            .as_string()
            .expect("as_string failed");

        // Verify that the JSON contains object-style fields
        assert!(json_string.contains(r#""fields""#));
        assert!(json_string.contains(r#""title":"Test Document""#));
        assert!(json_string.contains(r#""quillTag":"test-quill""#));

        // Parse back to verify it's a valid JSON object structure
        let parsed: serde_json::Value =
            serde_json::from_str(&json_string).expect("JSON parse failed");

        // Verify fields is an object
        assert!(parsed["fields"].is_object());
        assert_eq!(parsed["fields"]["title"], "Test Document");
        assert_eq!(parsed["fields"]["author"], "Test Author");
    }
}
