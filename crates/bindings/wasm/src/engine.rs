//! Quillmark WASM Engine - Simplified API

use crate::error::WasmError;
use crate::types::{ParsedDocument, RenderOptions, RenderResult};
use js_sys::{Array, Uint8Array};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
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
    inner: Arc<quillmark_core::Quill>,
}

#[wasm_bindgen]
pub struct RenderSession {
    inner: quillmark_core::RenderSession,
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
            inner: Arc::new(quill),
        })
    }
}

fn parse_markdown_impl(markdown: &str) -> Result<ParsedDocument, JsValue> {
    let parsed = quillmark_core::ParsedDocument::from_markdown(markdown)
        .map_err(WasmError::from)
        .map_err(|e| e.to_js_value())?;

    let quill_ref = parsed.quill_reference().to_string();

    let mut fields_obj = serde_json::Map::new();
    for (key, value) in parsed.fields() {
        fields_obj.insert(key.clone(), value.as_json().clone());
    }

    Ok(ParsedDocument {
        fields: serde_json::Value::Object(fields_obj),
        quill_ref,
    })
}

fn to_core_parsed(parsed: ParsedDocument) -> Result<quillmark_core::ParsedDocument, JsValue> {
    let mut fields = std::collections::HashMap::new();

    if let serde_json::Value::Object(obj) = parsed.fields {
        for (key, value) in obj {
            fields.insert(key, quillmark_core::value::QuillValue::from_json(value));
        }
    }

    let quill_ref =
        quillmark_core::version::QuillReference::from_str(&parsed.quill_ref).map_err(|e| {
            JsValue::from_str(&format!(
                "Invalid QUILL reference '{}': {}",
                parsed.quill_ref, e
            ))
        })?;

    Ok(quillmark_core::ParsedDocument::new(fields, quill_ref))
}

#[wasm_bindgen]
impl Quill {
    /// Render a document to final artifacts.
    #[wasm_bindgen(js_name = render)]
    pub fn render(
        &self,
        parsed: ParsedDocument,
        opts: RenderOptions,
    ) -> Result<RenderResult, JsValue> {
        let start = now_ms();
        let core_parsed = to_core_parsed(parsed).map_err(|e| {
            WasmError::from(format!("render: invalid ParsedDocument: {:?}", e)).to_js_value()
        })?;
        let rust_opts = quillmark_core::RenderOptions {
            output_format: opts.format.map(|f| f.into()),
            ppi: opts.ppi,
            pages: opts.pages,
        };
        let result = self
            .inner
            .render(core_parsed, &rust_opts)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            output_format: result.output_format.into(),
            render_time_ms: now_ms() - start,
        })
    }

    /// Open an iterative render session for page-selective rendering.
    #[wasm_bindgen(js_name = open)]
    pub fn open(&self, parsed: ParsedDocument) -> Result<RenderSession, JsValue> {
        let core_parsed = to_core_parsed(parsed).map_err(|e| {
            WasmError::from(format!("open: invalid ParsedDocument: {:?}", e)).to_js_value()
        })?;
        let session = self
            .inner
            .open(core_parsed)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(RenderSession { inner: session })
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
impl ParsedDocument {
    /// Parse markdown into a ParsedDocument.
    #[wasm_bindgen(js_name = fromMarkdown)]
    pub fn from_markdown(markdown: &str) -> Result<ParsedDocument, JsValue> {
        parse_markdown_impl(markdown)
    }
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
        let rust_opts = quillmark_core::RenderOptions {
            output_format: opts.format.map(|f| f.into()),
            ppi: opts.ppi,
            pages: opts.pages,
        };

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
