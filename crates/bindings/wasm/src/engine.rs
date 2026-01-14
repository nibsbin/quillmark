//! Quillmark WASM Engine - Simplified API

use crate::error::WasmError;
use crate::types::{OutputFormat, ParsedDocument, QuillInfo, RenderOptions, RenderResult};
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
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Quillmark {
    /// JavaScript constructor: `new Quillmark()`
    #[wasm_bindgen(constructor)]
    pub fn new() -> Quillmark {
        Quillmark {
            inner: quillmark::Quillmark::new(),
        }
    }

    /// Parse markdown into a ParsedDocument
    ///
    /// This is the first step in the workflow. The returned ParsedDocument contains
    /// the parsed YAML frontmatter fields and the quill_tag (from QUILL field or "__default__").
    #[wasm_bindgen(js_name = parseMarkdown)]
    pub fn parse_markdown(markdown: &str) -> Result<ParsedDocument, JsValue> {
        let parsed = quillmark_core::ParsedDocument::from_markdown(markdown)
            .map_err(WasmError::from)
            .map_err(|e| e.to_js_value())?;

        // Convert to WASM type
        let quill_tag = parsed.quill_tag().to_string();

        // Convert fields HashMap to JSON
        let mut fields_obj = serde_json::Map::new();
        for (key, value) in parsed.fields() {
            fields_obj.insert(key.clone(), value.as_json().clone());
        }
        let fields = serde_json::Value::Object(fields_obj);

        Ok(ParsedDocument { fields, quill_tag })
    }

    /// Register a Quill template bundle
    ///
    /// Accepts either a JSON string or a JsValue object representing the Quill file tree.
    /// Validation happens automatically on registration.
    #[wasm_bindgen(js_name = registerQuill)]
    pub fn register_quill(&mut self, quill_json: JsValue) -> Result<QuillInfo, JsValue> {
        // Convert JsValue to JSON string
        let json_str = if quill_json.is_string() {
            quill_json.as_string().ok_or_else(|| {
                WasmError::from("Failed to convert JsValue to string").to_js_value()
            })?
        } else {
            js_sys::JSON::stringify(&quill_json)
                .map_err(|e| {
                    WasmError::from(format!("Failed to serialize Quill JSON: {:?}", e))
                        .to_js_value()
                })?
                .as_string()
                .ok_or_else(|| WasmError::from("Failed to convert JSON to string").to_js_value())?
        };

        // Parse and validate Quill
        let quill = quillmark_core::Quill::from_json(&json_str)
            .map_err(|e| WasmError::from(format!("Failed to parse Quill: {}", e)).to_js_value())?;
        let name = quill.name.clone();

        // Register with backend validation
        self.inner
            .register_quill(quill)
            .map_err(|e| WasmError::from(e).to_js_value())?;

        // Return full quill info
        self.get_quill_info(&name)
    }

    /// Get shallow information about a registered Quill
    ///
    /// This returns metadata, backend info, field schemas, and supported formats
    /// that consumers need to configure render options for the next step.
    #[wasm_bindgen(js_name = getQuillInfo)]
    pub fn get_quill_info(&self, name: &str) -> Result<QuillInfo, JsValue> {
        self.fetch_quill_info(name)
    }

    fn fetch_quill_info(&self, name: &str) -> Result<QuillInfo, JsValue> {
        let quill = self.inner.get_quill(name).ok_or_else(|| {
            WasmError::from(format!("Quill '{}' not registered", name)).to_js_value()
        })?;

        // Get backend ID
        let backend_id = &quill.backend;

        // Create workflow to get supported formats
        let workflow = self.inner.workflow(name).map_err(|e| {
            WasmError::from(format!(
                "Failed to create workflow for quill '{}': {}",
                name, e
            ))
            .to_js_value()
        })?;

        let supported_formats: Vec<OutputFormat> = workflow
            .supported_formats()
            .iter()
            .map(|&f| f.into())
            .collect();

        // Convert metadata to serde_json::Value (plain JavaScript object)
        let mut metadata_obj = serde_json::Map::new();
        for (key, value) in &quill.metadata {
            metadata_obj.insert(key.clone(), value.as_json().clone());
        }
        let metadata_json = serde_json::Value::Object(metadata_obj);

        // Convert defaults to serde_json::Value (plain JavaScript object)
        let mut defaults_obj = serde_json::Map::new();
        for (key, value) in quill.extract_defaults() {
            defaults_obj.insert(key.clone(), value.as_json().clone());
        }
        let defaults_json = serde_json::Value::Object(defaults_obj);

        // Convert examples to serde_json::Value (plain JavaScript object with arrays)
        let mut examples_obj = serde_json::Map::new();
        for (key, values) in quill.extract_examples() {
            let examples_array: Vec<serde_json::Value> =
                values.iter().map(|v| v.as_json().clone()).collect();
            examples_obj.insert(key.clone(), serde_json::Value::Array(examples_array));
        }
        let examples_json = serde_json::Value::Object(examples_obj);

        // Prepare schema (always return full schema)
        let schema_json = quill.schema.clone().as_json().clone();

        Ok(QuillInfo {
            name: quill.name.clone(),
            backend: backend_id.clone(),
            metadata: metadata_json,
            example: quill.example.clone(),
            schema: schema_json,
            defaults: defaults_json,
            examples: examples_json,
            supported_formats,
        })
    }

    /// Get the stripped JSON schema of a Quill (removes UI metadata)
    ///
    /// This returns the schema in a format suitable for feeding to LLMs or
    /// other consumers that don't need the UI configuration "x-ui" fields.
    #[wasm_bindgen(js_name = getStrippedSchema)]
    pub fn get_stripped_schema(&self, name: &str) -> Result<JsValue, JsValue> {
        let quill = self.inner.get_quill(name).ok_or_else(|| {
            WasmError::from(format!("Quill '{}' not registered", name)).to_js_value()
        })?;

        // Clone the schema and strip it
        // We use the same logic as QuillInfo::get_stripped_schema but apply it directly
        // to avoid round-tripping through QuillInfo
        let mut schema_json = quill.schema.clone().as_json().clone();
        quillmark_core::schema::strip_schema_fields(&mut schema_json, &["x-ui"]);

        // Convert serde_json::Value to JsValue via JSON string to ensure clean object conversion
        let json_str = serde_json::to_string(&schema_json).map_err(|e| {
            WasmError::from(format!("Failed to serialize schema: {}", e)).to_js_value()
        })?;

        js_sys::JSON::parse(&json_str).map_err(|e| {
            WasmError::from(format!("Failed to parse JSON schema: {:?}", e)).to_js_value()
        })
    }

    /// Perform a dry run validation without backend compilation.
    ///
    /// Executes parsing, schema validation, and template composition to
    /// surface input errors quickly. Returns successfully on valid input,
    /// or throws an error with diagnostic payload on failure.
    ///
    /// The quill name is inferred from the markdown's QUILL tag (or defaults to "__default__").
    ///
    /// This is useful for fast feedback loops in LLM-driven document generation.
    #[wasm_bindgen(js_name = dryRun)]
    pub fn dry_run(&mut self, markdown: &str) -> Result<(), JsValue> {
        // Parse markdown first
        let parsed = quillmark_core::ParsedDocument::from_markdown(markdown)
            .map_err(WasmError::from)
            .map_err(|e| e.to_js_value())?;

        // Infer quill name from parsed document's quill_tag
        let quill_name = parsed.quill_tag();

        let workflow = self.inner.workflow(quill_name).map_err(|e| {
            WasmError::from(format!("Quill '{}' not found: {}", quill_name, e)).to_js_value()
        })?;

        workflow
            .dry_run(&parsed)
            .map_err(|e| WasmError::from(e).to_js_value())
    }

    /// Compile markdown to JSON data without rendering artifacts.
    ///
    /// This exposes the intermediate data structure that would be passed to the backend.
    /// Useful for debugging and validation.
    #[wasm_bindgen(js_name = compileData)]
    pub fn compile_data(&mut self, markdown: &str) -> Result<JsValue, JsValue> {
        // Parse markdown first
        let parsed = quillmark_core::ParsedDocument::from_markdown(markdown)
            .map_err(WasmError::from)
            .map_err(|e| e.to_js_value())?;

        // Infer quill name form parsed document's quill_tag
        let quill_name = parsed.quill_tag();

        let workflow = self.inner.workflow(quill_name).map_err(|e| {
            WasmError::from(format!("Quill '{}' not found: {}", quill_name, e)).to_js_value()
        })?;

        let json_data = workflow
            .compile_data(&parsed)
            .map_err(|e| WasmError::from(e).to_js_value())?;

        // Convert serde_json::Value to JsValue
        // We can stringify and parse, or use serde-wasm-bindgen (if available).
        // For simplicity/compatibility, let's use the JSON string approach via js_sys
        let json_str = serde_json::to_string(&json_data).map_err(|e| {
            WasmError::from(format!("Failed to serialize data: {}", e)).to_js_value()
        })?;

        js_sys::JSON::parse(&json_str).map_err(|e| {
            WasmError::from(format!("Failed to parse JSON data: {:?}", e)).to_js_value()
        })
    }

    /// Render a ParsedDocument to final artifacts (PDF, SVG, TXT)
    ///
    /// Uses the Quill specified in options.quill_name if provided,
    /// otherwise infers it from the ParsedDocument's quill_tag field.
    #[wasm_bindgen]
    pub fn render(
        &mut self,
        parsed: ParsedDocument,
        opts: RenderOptions,
    ) -> Result<RenderResult, JsValue> {
        // Determine which quill name to use (before consuming parsed)
        let quill_name_to_use = opts.quill_name.unwrap_or_else(|| parsed.quill_tag.clone());

        // Reconstruct a core ParsedDocument from the WASM type
        // Convert JSON value to HashMap<String, QuillValue>
        let fields_json = parsed.fields;
        let quill_tag = parsed.quill_tag; // Move quill_tag out

        let mut fields = std::collections::HashMap::new();

        if let serde_json::Value::Object(obj) = fields_json {
            for (key, value) in obj {
                fields.insert(key, quillmark_core::value::QuillValue::from_json(value));
            }
        }

        let parsed = quillmark_core::ParsedDocument::with_quill_tag(fields, quill_tag);

        // Load the workflow
        let mut workflow = self.inner.workflow(&quill_name_to_use).map_err(|e| {
            WasmError::from(format!("Quill '{}' not found: {}", quill_name_to_use, e)).to_js_value()
        })?;

        // Add assets if provided
        if let Some(serde_json::Value::Object(assets_map)) = opts.assets {
            // assets is now a serde_json::Value representing a plain JavaScript object
            // We need to convert it to an iterator of (filename, bytes)
            for (filename, value) in assets_map {
                // Extract bytes from the value
                // Bytes can be either an array of numbers or a Uint8Array
                let bytes = if let Some(arr) = value.as_array() {
                    // Array of numbers [0, 1, 2, ...]
                    arr.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect::<Vec<u8>>()
                } else {
                    return Err(WasmError::from(format!(
                        "Invalid asset format for '{}': expected byte array",
                        filename
                    ))
                    .to_js_value());
                };

                workflow.add_asset(filename, bytes).map_err(|e| {
                    WasmError::from(format!("Failed to add asset: {}", e)).to_js_value()
                })?;
            }
        }

        let start = now_ms();

        let output_format = opts.format.map(|f| f.into());
        let result = workflow
            .render(&parsed, output_format)
            .map_err(|e| WasmError::from(e).to_js_value())?;

        Ok(RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            output_format: result.output_format.into(),
            render_time_ms: now_ms() - start,
        })
    }

    /// List registered Quill names
    #[wasm_bindgen(js_name = listQuills)]
    pub fn list_quills(&self) -> Vec<String> {
        self.inner
            .registered_quills()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Unregister a Quill (free memory)
    #[wasm_bindgen(js_name = unregisterQuill)]
    pub fn unregister_quill(&mut self, name: &str) {
        self.inner.unregister_quill(name);
    }
}
