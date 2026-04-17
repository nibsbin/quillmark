//! Quillmark WASM Engine - Simplified API

use crate::error::WasmError;
use crate::types::{
    CompileOptions, OutputFormat, ParsedDocument, QuillInfo, RenderOptions, RenderPagesOptions,
    RenderResult,
};
use js_sys::{Array, Object, Uint8Array};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use wasm_bindgen::JsCast;
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

/// Opaque, shareable Quill handle.
#[wasm_bindgen]
pub struct Quill {
    inner: Arc<quillmark_core::Quill>,
}

#[wasm_bindgen]
pub struct CompiledDocument {
    backend: Arc<dyn quillmark_core::Backend>,
    inner: quillmark_core::CompiledDocument,
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
    /// the parsed YAML frontmatter fields and the quill_ref from QUILL.
    #[wasm_bindgen(js_name = parseMarkdown)]
    pub fn parse_markdown(markdown: &str) -> Result<ParsedDocument, JsValue> {
        let parsed = quillmark_core::ParsedDocument::from_markdown(markdown)
            .map_err(WasmError::from)
            .map_err(|e| e.to_js_value())?;

        // Convert to WASM type
        let quill_ref = parsed.quill_reference().to_string();

        // Convert fields HashMap to JSON
        let mut fields_obj = serde_json::Map::new();
        for (key, value) in parsed.fields() {
            fields_obj.insert(key.clone(), value.as_json().clone());
        }
        let fields = serde_json::Value::Object(fields_obj);

        Ok(ParsedDocument { fields, quill_ref })
    }

    /// Register a pre-constructed Quill handle.
    #[wasm_bindgen(js_name = registerQuill)]
    pub fn register_quill(&mut self, quill: &Quill) -> Result<QuillInfo, JsValue> {
        let quill = quill.inner.as_ref().clone();
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
        for (key, value) in quill.config.defaults() {
            defaults_obj.insert(key.clone(), value.as_json().clone());
        }
        let defaults_json = serde_json::Value::Object(defaults_obj);

        // Convert examples to serde_json::Value (plain JavaScript object with arrays)
        let mut examples_obj = serde_json::Map::new();
        for (key, values) in quill.config.examples() {
            let examples_array: Vec<serde_json::Value> =
                values.iter().map(|v| v.as_json().clone()).collect();
            examples_obj.insert(key.clone(), serde_json::Value::Array(examples_array));
        }
        let examples_json = serde_json::Value::Object(examples_obj);

        let schema_yaml = quill.config.public_schema_yaml().map_err(|e| {
            WasmError::from(format!("Failed to serialize schema: {}", e)).to_js_value()
        })?;

        Ok(QuillInfo {
            name: quill.name.clone(),
            backend: backend_id.clone(),
            metadata: metadata_json,
            example: quill.example.clone(),
            schema: schema_yaml,
            defaults: defaults_json,
            examples: examples_json,
            supported_formats,
        })
    }

    /// Get the public YAML schema contract for a registered quill.
    #[wasm_bindgen(js_name = getQuillSchema)]
    pub fn get_quill_schema(&self, name: &str) -> Result<String, JsValue> {
        let quill = self.inner.get_quill(name).ok_or_else(|| {
            WasmError::from(format!("Quill '{}' not registered", name)).to_js_value()
        })?;
        quill
            .config
            .public_schema_yaml()
            .map_err(|e| WasmError::from(format!("schema serialization: {}", e)).to_js_value())
    }

    /// Perform a dry run validation without backend compilation.
    ///
    /// Executes parsing, schema validation, and template composition to
    /// surface input errors quickly. Returns successfully on valid input,
    /// or throws an error with diagnostic payload on failure.
    ///
    /// The quill name is read from the markdown's required QUILL tag.
    ///
    /// This is useful for fast feedback loops in LLM-driven document generation.
    #[wasm_bindgen(js_name = dryRun)]
    pub fn dry_run(&mut self, markdown: &str) -> Result<(), JsValue> {
        // Parse markdown first
        let parsed = quillmark_core::ParsedDocument::from_markdown(markdown)
            .map_err(WasmError::from)
            .map_err(|e| e.to_js_value())?;

        // Read quill reference from parsed document
        let quill_ref = parsed.quill_reference().to_string();

        let workflow = self.inner.workflow(quill_ref.as_str()).map_err(|e| {
            WasmError::from(format!("Quill '{}' not found: {}", quill_ref, e)).to_js_value()
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

        // Read quill reference from parsed document
        let quill_ref = parsed.quill_reference().to_string();

        let workflow = self.inner.workflow(quill_ref.as_str()).map_err(|e| {
            WasmError::from(format!("Quill '{}' not found: {}", quill_ref, e)).to_js_value()
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
    /// Uses the Quill specified in the ParsedDocument's quill_ref field.
    #[wasm_bindgen]
    pub fn render(
        &mut self,
        parsed: ParsedDocument,
        opts: RenderOptions,
    ) -> Result<RenderResult, JsValue> {
        let quill_ref_to_use = parsed.quill_ref.clone();
        let parsed = Self::to_core_parsed(parsed)?;

        // Load the workflow
        let mut workflow = self.inner.workflow(&quill_ref_to_use).map_err(|e| {
            WasmError::from(format!("Quill '{}' not found: {}", quill_ref_to_use, e)).to_js_value()
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
            .render_with_options(&parsed, output_format, opts.ppi)
            .map_err(|e| WasmError::from(e).to_js_value())?;

        Ok(RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            output_format: result.output_format.into(),
            render_time_ms: now_ms() - start,
        })
    }

    /// Compile a parsed document into an opaque compiled document handle.
    #[wasm_bindgen]
    pub fn compile(
        &mut self,
        parsed: ParsedDocument,
        opts: Option<CompileOptions>,
    ) -> Result<CompiledDocument, JsValue> {
        let _opts = opts.unwrap_or_default();
        let quill_ref_to_use = parsed.quill_ref.clone();
        let parsed = Self::to_core_parsed(parsed)?;

        let workflow = self.inner.workflow(&quill_ref_to_use).map_err(|e| {
            WasmError::from(format!("Quill '{}' not found: {}", quill_ref_to_use, e)).to_js_value()
        })?;

        let backend = workflow.backend();

        let compiled = workflow
            .compile(&parsed)
            .map_err(|e| WasmError::from(e).to_js_value())?;

        Ok(CompiledDocument {
            backend,
            inner: compiled,
        })
    }

    /// Resolve a Quill reference to a registered Quill, or null if not available
    ///
    /// Accepts a quill reference string like "resume-template", "resume-template@2",
    /// or "resume-template@2.1.0". Returns QuillInfo if the engine can resolve it
    /// locally, or null if an external fetch is needed.
    #[wasm_bindgen(js_name = resolveQuill)]
    pub fn resolve_quill(&self, quill_ref: &str) -> JsValue {
        match self.fetch_quill_info(quill_ref) {
            Ok(info) => {
                let serializer =
                    serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
                use serde::Serialize;
                info.serialize(&serializer).unwrap_or(JsValue::NULL)
            }
            Err(_) => JsValue::NULL,
        }
    }

    /// List registered Quills with their exact versions
    ///
    /// Returns strings in the format "name@version" (e.g. "resume-template@2.1.0")
    #[wasm_bindgen(js_name = listQuills)]
    pub fn list_quills(&self) -> Vec<String> {
        self.inner.registered_quill_versions()
    }

    /// Unregister a Quill by name or specific version
    ///
    /// If a base name is provided (e.g., "my-quill"), all versions of that quill are freed.
    /// If a versioned name is provided (e.g., "my-quill@2.1.0"), only that specific version is freed.
    /// Returns true if something was unregistered, false if not found.
    #[wasm_bindgen(js_name = unregisterQuill)]
    pub fn unregister_quill(&mut self, name_or_ref: &str) -> bool {
        self.inner.unregister_quill(name_or_ref)
    }

    fn to_core_parsed(parsed: ParsedDocument) -> Result<quillmark_core::ParsedDocument, JsValue> {
        let mut fields = std::collections::HashMap::new();

        if let serde_json::Value::Object(obj) = parsed.fields {
            for (key, value) in obj {
                fields.insert(key, quillmark_core::value::QuillValue::from_json(value));
            }
        }

        let quill_ref = quillmark_core::version::QuillReference::from_str(&parsed.quill_ref)
            .map_err(|e| {
                JsValue::from_str(&format!(
                    "Invalid QUILL reference '{}': {}",
                    parsed.quill_ref, e
                ))
            })?;

        Ok(quillmark_core::ParsedDocument::new(fields, quill_ref))
    }
}

#[wasm_bindgen]
impl Quill {
    /// Parse and validate a Quill from JSON string or object.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(source: JsValue) -> Result<Quill, JsValue> {
        let json_str = if source.is_string() {
            source
                .as_string()
                .ok_or_else(|| WasmError::from("Failed to convert source to string").to_js_value())?
        } else {
            js_sys::JSON::stringify(&source)
                .map_err(|e| {
                    WasmError::from(format!("Failed to serialize Quill JSON: {:?}", e))
                        .to_js_value()
                })?
                .as_string()
                .ok_or_else(|| WasmError::from("Failed to convert JSON to string").to_js_value())?
        };

        let quill = quillmark_core::Quill::from_json(&json_str)
            .map_err(|e| WasmError::from(format!("Failed to parse Quill: {}", e)).to_js_value())?;

        Ok(Quill {
            inner: Arc::new(quill),
        })
    }

    /// Build and validate a Quill from a flat path -> bytes tree.
    #[wasm_bindgen(js_name = fromTree)]
    pub fn from_tree(tree: JsValue) -> Result<Quill, JsValue> {
        let root = file_tree_from_js_tree(&tree)?;
        let quill = quillmark_core::Quill::from_tree(root)
            .map_err(|e| WasmError::from(format!("Failed to parse Quill: {}", e)).to_js_value())?;

        Ok(Quill {
            inner: Arc::new(quill),
        })
    }
}

fn file_tree_from_js_tree(tree: &JsValue) -> Result<quillmark_core::FileTreeNode, JsValue> {
    let entries = js_tree_entries(tree)?;
    let mut root = quillmark_core::FileTreeNode::Directory {
        files: HashMap::new(),
    };

    for (path, value) in entries {
        let bytes = js_bytes_for_tree_entry(&path, value)?;
        root.insert(path.as_str(), quillmark_core::FileTreeNode::File { contents: bytes })
            .map_err(|e| {
                WasmError::from(format!("Invalid tree path '{}': {}", path, e)).to_js_value()
            })?;
    }

    Ok(root)
}

fn js_tree_entries(tree: &JsValue) -> Result<Vec<(String, JsValue)>, JsValue> {
    if tree.is_null() || tree.is_undefined() {
        return Err(WasmError::from("fromTree requires a Map or plain object").to_js_value());
    }

    let mut entries: Vec<(String, JsValue)> = Vec::new();

    if tree.is_instance_of::<js_sys::Map>() {
        let map = tree.clone().unchecked_into::<js_sys::Map>();
        let iter = js_sys::try_iter(&map.entries())
            .map_err(|e| {
                WasmError::from(format!("Failed to iterate Map entries: {:?}", e)).to_js_value()
            })?
            .ok_or_else(|| WasmError::from("Map entries are not iterable").to_js_value())?;

        for entry in iter {
            let pair = entry.map_err(|e| {
                WasmError::from(format!("Failed to read Map entry: {:?}", e)).to_js_value()
            })?;
            let pair = Array::from(&pair);
            let path = pair
                .get(0)
                .as_string()
                .ok_or_else(|| WasmError::from("fromTree Map key must be a string").to_js_value())?;
            let value = pair.get(1);
            entries.push((path, value));
        }
        return Ok(entries);
    }

    if tree.is_object() {
        let obj = tree.clone().unchecked_into::<Object>();
        for pair in Object::entries(&obj).iter() {
            let pair = Array::from(&pair);
            let path = pair
                .get(0)
                .as_string()
                .ok_or_else(|| WasmError::from("fromTree object key must be a string").to_js_value())?;
            let value = pair.get(1);
            entries.push((path, value));
        }
        return Ok(entries);
    }

    Err(WasmError::from("fromTree requires a Map or plain object").to_js_value())
}

fn js_bytes_for_tree_entry(path: &str, value: JsValue) -> Result<Vec<u8>, JsValue> {
    if !value.is_instance_of::<Uint8Array>() {
        return Err(WasmError::from(format!(
            "Invalid tree entry '{}': expected Uint8Array value",
            path
        ))
        .to_js_value());
    }

    let bytes = value.unchecked_into::<Uint8Array>();
    Ok(bytes.to_vec())
}

#[wasm_bindgen]
impl CompiledDocument {
    /// Number of pages in this compiled document.
    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> usize {
        self.inner.page_count
    }

    /// Render selected pages. `pages = null/undefined` renders all pages.
    #[wasm_bindgen(js_name = renderPages)]
    pub fn render_pages(
        &self,
        pages: Option<Vec<u32>>,
        opts: RenderPagesOptions,
    ) -> Result<RenderResult, JsValue> {
        let page_indices = pages.map(|v| v.into_iter().map(|i| i as usize).collect::<Vec<_>>());
        let start = now_ms();

        let result = self
            .backend
            .render_pages(
                &self.inner,
                page_indices.as_deref(),
                opts.format.into(),
                opts.ppi,
            )
            .map_err(|e| WasmError::from(e).to_js_value())?;

        Ok(RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            output_format: result.output_format.into(),
            render_time_ms: now_ms() - start,
        })
    }
}
