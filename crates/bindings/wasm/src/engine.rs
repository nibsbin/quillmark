//! Quillmark WASM Engine - Simplified API

use crate::error::WasmError;
use crate::types::{ParsedDocument, RenderOptions, RenderPagesOptions, RenderResult};
use js_sys::{Array, Object, Uint8Array};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    fn console_warn(s: &str);
}

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

    /// Parse markdown into a ParsedDocument.
    ///
    /// @deprecated Use `ParsedDocument.fromMarkdown()` instead.
    #[wasm_bindgen(js_name = parseMarkdown)]
    pub fn parse_markdown(markdown: &str) -> Result<ParsedDocument, JsValue> {
        console_warn(
            "[quillmark] Quillmark.parseMarkdown() is deprecated; use ParsedDocument.fromMarkdown() instead",
        );
        parse_markdown_impl(markdown)
    }

    /// Load a quill from a file tree and attach the appropriate backend.
    ///
    /// The tree must be a `Map<string, Uint8Array>` or `Record<string, Uint8Array>`.
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
    ///
    /// Input may be a markdown string or a `ParsedDocument` object.
    #[wasm_bindgen(js_name = render)]
    pub fn render(&self, input: JsValue, opts: RenderOptions) -> Result<RenderResult, JsValue> {
        let start = now_ms();
        let core_input = js_value_to_quill_input(input)?;
        let rust_opts = quillmark_core::RenderOptions {
            output_format: opts.format.map(|f| f.into()),
            ppi: opts.ppi,
        };
        let result = self
            .inner
            .render(core_input, &rust_opts)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            output_format: result.output_format.into(),
            render_time_ms: now_ms() - start,
        })
    }

    /// Compile a document to an opaque compiled document handle for page-selective rendering.
    #[wasm_bindgen(js_name = compile)]
    pub fn compile(&self, input: JsValue) -> Result<CompiledDocument, JsValue> {
        let core_input = js_value_to_quill_input(input)?;
        let backend = self.inner.backend().ok_or_else(|| {
            WasmError::from("Quill has no backend; use engine.quill(...)").to_js_value()
        })?;
        let compiled = self
            .inner
            .compile(core_input)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(CompiledDocument {
            backend: Arc::clone(backend),
            inner: compiled,
        })
    }
}

fn js_value_to_quill_input(input: JsValue) -> Result<quillmark_core::QuillInput, JsValue> {
    if let Some(s) = input.as_string() {
        return Ok(quillmark_core::QuillInput::Markdown(s));
    }
    // Try to deserialize as ParsedDocument (plain JS object)
    let parsed: ParsedDocument = serde_wasm_bindgen::from_value(input).map_err(|e| {
        WasmError::from(format!(
            "render: input must be a string (markdown) or ParsedDocument: {}",
            e
        ))
        .to_js_value()
    })?;
    let core_parsed = to_core_parsed(parsed).map_err(|e| {
        WasmError::from(format!("render: invalid ParsedDocument: {:?}", e)).to_js_value()
    })?;
    Ok(quillmark_core::QuillInput::Parsed(core_parsed))
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
    if tree.is_null() || tree.is_undefined() {
        return Err(WasmError::from("quill requires a Map or plain object").to_js_value());
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
                .ok_or_else(|| WasmError::from("quill Map key must be a string").to_js_value())?;
            let value = pair.get(1);
            entries.push((path, value));
        }
        return Ok(entries);
    }

    if tree.is_instance_of::<js_sys::Array>() {
        return Err(
            WasmError::from("quill requires a Map or plain object, not an Array").to_js_value(),
        );
    }
    if tree.is_instance_of::<Uint8Array>() {
        return Err(WasmError::from(
            "quill requires a Map or plain object, not a Uint8Array; \
                 did you mean to pass a Map<string, Uint8Array>?",
        )
        .to_js_value());
    }

    if tree.is_object() {
        let obj = tree.clone().unchecked_into::<Object>();
        for pair in Object::entries(&obj).iter() {
            let pair = Array::from(&pair);
            let path = pair.get(0).as_string().ok_or_else(|| {
                WasmError::from("quill object key must be a string").to_js_value()
            })?;
            let value = pair.get(1);
            entries.push((path, value));
        }
        return Ok(entries);
    }

    Err(WasmError::from("quill requires a Map or plain object").to_js_value())
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
