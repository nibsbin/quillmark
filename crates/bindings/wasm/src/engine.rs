//! Quillmark WASM Engine - Simplified API

use crate::error::WasmError;
use crate::types::{Diagnostic, RenderOptions, RenderResult};
use js_sys::{Array, Uint8Array};
use serde::Serialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

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
#[wasm_bindgen]
pub struct Quillmark {
    inner: quillmark::Quillmark,
}

/// Opaque, shareable Quill handle.
#[wasm_bindgen]
pub struct Quill {
    inner: std::sync::Arc<quillmark_core::Quill>,
}

#[wasm_bindgen]
pub struct RenderSession {
    inner: quillmark_core::RenderSession,
}

/// Typed in-memory Quillmark document.
///
/// Created via `Document.fromMarkdown(markdown)`. Exposes:
/// - `quillRef` (string)
/// - `frontmatter` (JS object/Record)
/// - `body` (string)
/// - `cards` (array of Card objects)
/// - `warnings` (array of Diagnostic objects)
///
/// `toMarkdown()` is a stub — it throws with a "not yet implemented (phase 4)"
/// message until the emitter is implemented in Phase 4.
#[wasm_bindgen]
pub struct Document {
    inner: quillmark_core::Document,
    /// Parse-time warnings (e.g. near-miss sentinel lints).
    parse_warnings: Vec<quillmark_core::Diagnostic>,
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

    /// Load a quill from a file tree and attach the appropriate backend.
    ///
    /// The tree must be a `Map<string, Uint8Array>`.
    #[wasm_bindgen(js_name = quill)]
    pub fn quill(&self, tree: JsValue) -> Result<Quill, JsValue> {
        let root = file_tree_from_js_tree(&tree)?;
        let quill = self
            .inner
            .quill(root)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(Quill {
            inner: std::sync::Arc::new(quill),
        })
    }
}

#[wasm_bindgen]
impl Quill {
    /// Render a document to final artifacts.
    #[wasm_bindgen(js_name = render)]
    pub fn render(
        &self,
        doc: Document,
        opts: RenderOptions,
    ) -> Result<RenderResult, JsValue> {
        let start = now_ms();
        let parse_warnings = doc.parse_warnings.clone();
        let rust_opts: quillmark_core::RenderOptions = opts.into();
        let result = self
            .inner
            .render(doc.inner, &rust_opts)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        let mut warnings: Vec<Diagnostic> = parse_warnings.into_iter().map(Into::into).collect();
        warnings.extend(result.warnings.into_iter().map(Into::into));
        Ok(RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings,
            output_format: result.output_format.into(),
            render_time_ms: now_ms() - start,
        })
    }

    /// Open an iterative render session for page-selective rendering.
    #[wasm_bindgen(js_name = open)]
    pub fn open(&self, doc: Document) -> Result<RenderSession, JsValue> {
        let session = self
            .inner
            .open(doc.inner)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(RenderSession { inner: session })
    }
}

#[wasm_bindgen]
impl Document {
    /// Parse markdown into a typed Document.
    ///
    /// Returns the document with any parse-time warnings accessible via `.warnings`.
    /// Throws on parse errors.
    #[wasm_bindgen(js_name = fromMarkdown)]
    pub fn from_markdown(markdown: &str) -> Result<Document, JsValue> {
        let output = quillmark_core::Document::from_markdown_with_warnings(markdown)
            .map_err(WasmError::from)
            .map_err(|e| e.to_js_value())?;

        Ok(Document {
            inner: output.document,
            parse_warnings: output.warnings,
        })
    }

    /// Emit canonical Quillmark Markdown.
    ///
    /// **Not yet implemented.** Throws with a clear message until Phase 4.
    #[wasm_bindgen(js_name = toMarkdown)]
    pub fn to_markdown(&self) -> Result<String, JsValue> {
        Err(WasmError::from("toMarkdown not yet implemented (phase 4)").to_js_value())
    }

    /// The QUILL reference string (e.g. `"usaf_memo@0.1"`).
    #[wasm_bindgen(getter, js_name = quillRef)]
    pub fn quill_ref(&self) -> String {
        self.inner.quill_reference().to_string()
    }

    /// Typed YAML frontmatter fields as a JS object (no QUILL, BODY, or CARDS keys).
    #[wasm_bindgen(getter, js_name = frontmatter)]
    pub fn frontmatter(&self) -> JsValue {
        let mut map = serde_json::Map::new();
        for (k, v) in self.inner.frontmatter() {
            map.insert(k.clone(), v.as_json().clone());
        }
        let val = serde_json::Value::Object(map);
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        val.serialize(&serializer).unwrap_or(JsValue::UNDEFINED)
    }

    /// Global Markdown body between frontmatter and the first card.
    ///
    /// Empty string when no body is present.
    #[wasm_bindgen(getter, js_name = body)]
    pub fn body(&self) -> String {
        self.inner.body().to_string()
    }

    /// Ordered list of card blocks as JS objects with `tag`, `fields`, and `body`.
    #[wasm_bindgen(getter, js_name = cards)]
    pub fn cards(&self) -> JsValue {
        let cards: Vec<serde_json::Value> = self
            .inner
            .cards()
            .iter()
            .map(|card| {
                let mut fields_map = serde_json::Map::new();
                for (k, v) in card.fields() {
                    fields_map.insert(k.clone(), v.as_json().clone());
                }
                serde_json::json!({
                    "tag": card.tag(),
                    "fields": serde_json::Value::Object(fields_map),
                    "body": card.body(),
                })
            })
            .collect();
        let val = serde_json::Value::Array(cards);
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        val.serialize(&serializer).unwrap_or(JsValue::UNDEFINED)
    }

    /// Non-fatal parse-time warnings as a JS array of Diagnostic objects.
    #[wasm_bindgen(getter, js_name = warnings)]
    pub fn warnings(&self) -> JsValue {
        let diags: Vec<Diagnostic> = self
            .parse_warnings
            .iter()
            .map(|d| d.clone().into())
            .collect();
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        diags.serialize(&serializer).unwrap_or(JsValue::UNDEFINED)
    }
}

fn file_tree_from_js_tree(tree: &JsValue) -> Result<quillmark_core::FileTreeNode, JsValue> {
    let entries = js_tree_entries(tree)?;
    let mut root = quillmark_core::FileTreeNode::Directory {
        files: HashMap::new(),
    };

    for (path, value) in entries {
        let bytes = js_bytes_for_tree_entry(&path, value)?;
        root.insert(
            path.as_str(),
            quillmark_core::FileTreeNode::File { contents: bytes },
        )
        .map_err(|e| {
            WasmError::from(format!("Invalid tree path '{}': {}", path, e)).to_js_value()
        })?;
    }

    Ok(root)
}

fn js_tree_entries(tree: &JsValue) -> Result<Vec<(String, JsValue)>, JsValue> {
    if !tree.is_instance_of::<js_sys::Map>() {
        return Err(WasmError::from("quill requires a Map<string, Uint8Array>").to_js_value());
    }

    let map = tree.clone().unchecked_into::<js_sys::Map>();
    let iter = js_sys::try_iter(&map.entries())
        .map_err(|e| {
            WasmError::from(format!("Failed to iterate Map entries: {:?}", e)).to_js_value()
        })?
        .ok_or_else(|| WasmError::from("Map entries are not iterable").to_js_value())?;

    let mut entries: Vec<(String, JsValue)> = Vec::new();
    for entry in iter {
        let pair = entry.map_err(|e| {
            WasmError::from(format!("Failed to read Map entry: {:?}", e)).to_js_value()
        })?;
        let pair = Array::from(&pair);
        let path = pair
            .get(0)
            .as_string()
            .ok_or_else(|| WasmError::from("quill Map key must be a string").to_js_value())?;
        let value = pair.get(1);
        entries.push((path, value));
    }
    Ok(entries)
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
impl RenderSession {
    /// Number of pages in this render session.
    #[wasm_bindgen(getter, js_name = pageCount)]
    pub fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    /// Render all or selected pages from this session.
    #[wasm_bindgen(js_name = render)]
    pub fn render(&self, opts: RenderOptions) -> Result<RenderResult, JsValue> {
        let start = now_ms();
        let rust_opts: quillmark_core::RenderOptions = opts.into();

        let result = self
            .inner
            .render(&rust_opts)
            .map_err(|e| WasmError::from(e).to_js_value())?;

        Ok(RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            output_format: result.output_format.into(),
            render_time_ms: now_ms() - start,
        })
    }
}
