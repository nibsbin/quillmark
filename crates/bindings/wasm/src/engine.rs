//! Quillmark WASM Engine - Simplified API

use crate::error::WasmError;
use crate::types::{Diagnostic, RenderOptions, RenderResult};
use js_sys::{Array, Uint8Array};
use serde::{Deserialize, Serialize};
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
    inner: quillmark::Quill,
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
/// `toMarkdown()` emits canonical Quillmark Markdown that round-trips back to
/// an equal `Document` by value and by type.
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
    /// Accepts either a `Map<string, Uint8Array>` or a plain object
    /// (`Record<string, Uint8Array>`). Plain objects are walked via
    /// `Object.entries` at the boundary; the Rust side sees a single
    /// canonical shape.
    #[wasm_bindgen(js_name = quill)]
    pub fn quill(&self, tree: JsValue) -> Result<Quill, JsValue> {
        let root = file_tree_from_js_tree(&tree)?;
        let quill = self
            .inner
            .quill(root)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(Quill { inner: quill })
    }
}

#[wasm_bindgen]
impl Quill {
    /// Render a document to final artifacts.
    #[wasm_bindgen(js_name = render)]
    pub fn render(
        &self,
        doc: &Document,
        opts: Option<RenderOptions>,
    ) -> Result<RenderResult, JsValue> {
        let start = now_ms();
        let rust_opts: quillmark_core::RenderOptions = opts.unwrap_or_default().into();
        let result = self
            .inner
            .render(&doc.inner, &rust_opts)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        let mut warnings: Vec<Diagnostic> =
            doc.parse_warnings.iter().cloned().map(Into::into).collect();
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
    pub fn open(&self, doc: &Document) -> Result<RenderSession, JsValue> {
        let session = self
            .inner
            .open(&doc.inner)
            .map_err(|e| WasmError::from(e).to_js_value())?;
        Ok(RenderSession { inner: session })
    }

    /// The resolved backend identifier (e.g. `"typst"`).
    #[wasm_bindgen(getter, js_name = backendId)]
    pub fn backend_id(&self) -> String {
        self.inner.backend_id().to_string()
    }

    /// Read-only snapshot of the loaded `Quill.yaml`.
    ///
    /// Returns a plain JS object with `name`, `backend`, `description`,
    /// `version`, `author`, optional `example` (content of the example
    /// markdown, when the quill ships one), `supportedFormats` (backend's
    /// output formats as lowercase strings), `schema` (the raw main-card
    /// field schema as parsed from YAML — consumers that need validation
    /// run their own validator against this), and any additional
    /// unstructured keys declared inside the `quill:` section.
    ///
    /// Equivalent by value for the lifetime of the handle; the quill is
    /// immutable once constructed.
    #[wasm_bindgen(getter, js_name = metadata)]
    pub fn metadata(&self) -> JsValue {
        let source = self.inner.source();
        let config = source.config();

        let mut obj = serde_json::Map::new();
        obj.insert(
            "name".to_string(),
            serde_json::Value::String(config.name.clone()),
        );
        obj.insert(
            "backend".to_string(),
            serde_json::Value::String(config.backend.clone()),
        );
        obj.insert(
            "description".to_string(),
            serde_json::Value::String(config.main().description.clone().unwrap_or_default()),
        );
        obj.insert(
            "version".to_string(),
            serde_json::Value::String(config.version.clone()),
        );
        obj.insert(
            "author".to_string(),
            serde_json::Value::String(config.author.clone()),
        );
        if let Some(example) = source.example() {
            obj.insert(
                "example".to_string(),
                serde_json::Value::String(example.to_string()),
            );
        }

        let formats: Vec<serde_json::Value> = self
            .inner
            .supported_formats()
            .iter()
            .map(|f| {
                let wasm_format: crate::types::OutputFormat = (*f).into();
                serde_json::to_value(wasm_format).unwrap_or(serde_json::Value::Null)
            })
            .collect();
        obj.insert(
            "supportedFormats".to_string(),
            serde_json::Value::Array(formats),
        );

        let mut schema = serde_json::Map::new();
        for (name, field) in &config.main().fields {
            schema.insert(
                name.clone(),
                serde_json::to_value(field).unwrap_or(serde_json::Value::Null),
            );
        }
        obj.insert("schema".to_string(), serde_json::Value::Object(schema));

        // Unstructured keys declared under `quill:` (excluding fields already
        // surfaced above).
        for (key, value) in source.metadata() {
            if matches!(
                key.as_str(),
                "name" | "backend" | "description" | "version" | "author"
            ) {
                continue;
            }
            if obj.contains_key(key) {
                continue;
            }
            obj.insert(key.clone(), value.as_json().clone());
        }

        let val = serde_json::Value::Object(obj);
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        val.serialize(&serializer).unwrap_or(JsValue::UNDEFINED)
    }

    /// Project a document through this quill's schema.
    ///
    /// Returns a plain JS object (not a class) that is immediately
    /// `JSON.stringify`-able. The shape mirrors [`FormProjection`]:
    ///
    /// ```json
    /// {
    ///   "main":  { "schema": {...}, "values": { "field": {...} } },
    ///   "cards": [ ... ],
    ///   "diagnostics": [ ... ]
    /// }
    /// ```
    ///
    /// **Snapshot semantics.** This is a read-only snapshot of the document
    /// at call time. Subsequent edits to `doc` require calling `projectForm`
    /// again.
    ///
    /// [`FormProjection`]: quillmark::form::FormProjection
    #[wasm_bindgen(js_name = projectForm)]
    pub fn project_form(&self, doc: &Document) -> Result<JsValue, JsValue> {
        let projection = quillmark::form::project_form(&self.inner, &doc.inner);
        let serializer = serde_wasm_bindgen::Serializer::new()
            .serialize_maps_as_objects(true)
            .serialize_missing_as_null(true);
        projection.serialize(&serializer).map_err(|e| {
            WasmError::from(format!("projectForm: serialization failed: {e}")).to_js_value()
        })
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
    /// Returns the document serialised as a Quillmark Markdown string.
    /// The output is type-fidelity round-trip safe: re-parsing the result
    /// produces a `Document` equal to `self` by value and by type.
    #[wasm_bindgen(js_name = toMarkdown)]
    pub fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    /// Return a fresh `Document` handle with the same parse state.
    ///
    /// Mutations on the returned handle do not affect the original and
    /// vice versa. Parse-time warnings are snapshotted alongside the
    /// document — they describe the original parse, not the edit
    /// history of either handle.
    #[wasm_bindgen(js_name = clone)]
    pub fn clone_doc(&self) -> Document {
        Document {
            inner: self.inner.clone(),
            parse_warnings: self.parse_warnings.clone(),
        }
    }

    /// The QUILL reference string (e.g. `"usaf_memo@0.1"`).
    #[wasm_bindgen(getter, js_name = quillRef)]
    pub fn quill_ref(&self) -> String {
        self.inner.quill_reference().to_string()
    }

    /// Typed YAML frontmatter fields as a JS object (no QUILL, BODY, or CARDS keys).
    ///
    /// Allocates and serializes on each call — cache locally if read in a hot loop.
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
    /// Trailing newlines are stripped — those are structural separators in
    /// the Markdown wire format, not content the consumer wrote.
    ///
    /// Empty string when no body is present.
    #[wasm_bindgen(getter, js_name = body)]
    pub fn body(&self) -> String {
        trim_body(self.inner.body())
    }

    /// Ordered list of card blocks as JS objects with `tag`, `fields`, and `body`.
    ///
    /// Allocates and serializes on each call — cache locally if read in a hot loop.
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
                    "body": trim_body(card.body()),
                })
            })
            .collect();
        let val = serde_json::Value::Array(cards);
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        val.serialize(&serializer).unwrap_or(JsValue::UNDEFINED)
    }

    /// Non-fatal parse-time warnings as a JS array of Diagnostic objects.
    ///
    /// Allocates and serializes on each call — cache locally if read in a hot loop.
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

    // ── Mutators ──────────────────────────────────────────────────────────────

    /// Set a frontmatter field.
    ///
    /// Throws an `Error` whose message includes the `EditError` variant name and
    /// details if `name` is reserved (`BODY`, `CARDS`, `QUILL`, `CARD`) or does
    /// not match `[a-z_][a-z0-9_]*`.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = setField)]
    pub fn set_field(&mut self, name: &str, value: JsValue) -> Result<(), JsValue> {
        let json: serde_json::Value = serde_wasm_bindgen::from_value(value).map_err(|e| {
            WasmError::from(format!("setField: invalid value: {}", e)).to_js_value()
        })?;
        let qv = quillmark_core::QuillValue::from_json(json);
        self.inner
            .set_field(name, qv)
            .map_err(|e| edit_error_to_js(&e))
    }

    /// Remove a frontmatter field, returning the removed value or `undefined`.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = removeField)]
    pub fn remove_field(&mut self, name: &str) -> JsValue {
        match self.inner.remove_field(name) {
            Some(v) => {
                let serializer =
                    serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
                v.as_json()
                    .serialize(&serializer)
                    .unwrap_or(JsValue::UNDEFINED)
            }
            None => JsValue::UNDEFINED,
        }
    }

    /// Replace the QUILL reference string.
    ///
    /// Throws if `ref_str` is not a valid `QuillReference`.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = setQuillRef)]
    pub fn set_quill_ref(&mut self, ref_str: &str) -> Result<(), JsValue> {
        let qr: quillmark_core::QuillReference = ref_str.parse().map_err(|e| {
            WasmError::from(format!(
                "setQuillRef: invalid reference '{}': {}",
                ref_str, e
            ))
            .to_js_value()
        })?;
        self.inner.set_quill_ref(qr);
        Ok(())
    }

    /// Replace the global Markdown body.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = replaceBody)]
    pub fn replace_body(&mut self, body: &str) {
        self.inner.replace_body(body);
    }

    /// Append a card to the end of the card list.
    ///
    /// `card` must be a JS object with a `tag` string field and optional
    /// `fields` (object) and `body` (string).
    ///
    /// Throws an `Error` if `card.tag` is not a valid tag name.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = pushCard)]
    pub fn push_card(&mut self, card: JsValue) -> Result<(), JsValue> {
        let core_card = js_value_to_card(&card)?;
        self.inner
            .push_card(core_card)
            .map_err(|e| edit_error_to_js(&e))
    }

    /// Insert a card at the given index.
    ///
    /// `index` must be in `0..=cards.length`. Out-of-range throws an `Error`.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = insertCard)]
    pub fn insert_card(&mut self, index: usize, card: JsValue) -> Result<(), JsValue> {
        let core_card = js_value_to_card(&card)?;
        self.inner
            .insert_card(index, core_card)
            .map_err(|e| edit_error_to_js(&e))
    }

    /// Remove the card at `index` and return it, or `undefined` if out of range.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = removeCard)]
    pub fn remove_card(&mut self, index: usize) -> JsValue {
        match self.inner.remove_card(index) {
            Some(card) => card_to_js_value(&card),
            None => JsValue::UNDEFINED,
        }
    }

    /// Move the card at `from` to position `to`.
    ///
    /// `from == to` is a no-op. Both indices must be in `0..cards.length`.
    /// Out-of-range throws an `Error`.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = moveCard)]
    pub fn move_card(&mut self, from: usize, to: usize) -> Result<(), JsValue> {
        self.inner
            .move_card(from, to)
            .map_err(|e| edit_error_to_js(&e))
    }

    /// Update a field on the card at `index`.
    ///
    /// Convenience method: equivalent to `doc.card_mut(index)?.set_field(name, value)`.
    ///
    /// Throws if `index` is out of range, `name` is reserved or invalid, or
    /// `value` cannot be serialized.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = updateCardField)]
    pub fn update_card_field(
        &mut self,
        index: usize,
        name: &str,
        value: JsValue,
    ) -> Result<(), JsValue> {
        let len = self.inner.cards().len();
        let card = self.inner.card_mut(index).ok_or_else(|| {
            edit_error_to_js(&quillmark_core::EditError::IndexOutOfRange { index, len })
        })?;
        let json: serde_json::Value = serde_wasm_bindgen::from_value(value).map_err(|e| {
            WasmError::from(format!("updateCardField: invalid value: {}", e)).to_js_value()
        })?;
        let qv = quillmark_core::QuillValue::from_json(json);
        card.set_field(name, qv).map_err(|e| edit_error_to_js(&e))
    }

    /// Replace the body of the card at `index`.
    ///
    /// Throws if `index` is out of range.
    ///
    /// Mutators never modify `warnings`.
    #[wasm_bindgen(js_name = updateCardBody)]
    pub fn update_card_body(&mut self, index: usize, body: &str) -> Result<(), JsValue> {
        let len = self.inner.cards().len();
        let card = self.inner.card_mut(index).ok_or_else(|| {
            edit_error_to_js(&quillmark_core::EditError::IndexOutOfRange { index, len })
        })?;
        card.set_body(body);
        Ok(())
    }
}

// ── Edit helpers ──────────────────────────────────────────────────────────────

/// Convert an [`quillmark_core::EditError`] into a JS `Error` value whose
/// message includes the variant name and details.
fn edit_error_to_js(err: &quillmark_core::EditError) -> JsValue {
    let variant = match err {
        quillmark_core::EditError::ReservedName(_) => "ReservedName",
        quillmark_core::EditError::InvalidFieldName(_) => "InvalidFieldName",
        quillmark_core::EditError::InvalidTagName(_) => "InvalidTagName",
        quillmark_core::EditError::IndexOutOfRange { .. } => "IndexOutOfRange",
    };
    WasmError::from(format!("[EditError::{}] {}", variant, err)).to_js_value()
}

/// Deserialise a JS object `{ tag: string, fields?: object, body?: string }`
/// into a [`quillmark_core::Card`].  Throws on invalid tag.
fn js_value_to_card(value: &JsValue) -> Result<quillmark_core::Card, JsValue> {
    #[derive(Deserialize)]
    struct CardInput {
        tag: String,
        #[serde(default)]
        fields: serde_json::Map<String, serde_json::Value>,
        #[serde(default)]
        body: String,
    }

    let input: CardInput = serde_wasm_bindgen::from_value(value.clone()).map_err(|e| {
        WasmError::from(format!("card must be {{ tag, fields?, body? }}: {}", e)).to_js_value()
    })?;

    // Validate tag via Card::new, then upgrade with fields and body.
    let mut card = quillmark_core::Card::new(input.tag).map_err(|e| edit_error_to_js(&e))?;

    for (k, v) in input.fields {
        let qv = quillmark_core::QuillValue::from_json(v);
        card.set_field(&k, qv).map_err(|e| edit_error_to_js(&e))?;
    }
    card.set_body(input.body);
    Ok(card)
}

/// Serialise a [`quillmark_core::Card`] to a JS value
/// `{ tag: string, fields: object, body: string }`.
fn card_to_js_value(card: &quillmark_core::Card) -> JsValue {
    let mut fields_map = serde_json::Map::new();
    for (k, v) in card.fields() {
        fields_map.insert(k.clone(), v.as_json().clone());
    }
    let json = serde_json::json!({
        "tag": card.tag(),
        "fields": serde_json::Value::Object(fields_map),
        "body": trim_body(card.body()),
    });
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    json.serialize(&serializer).unwrap_or(JsValue::UNDEFINED)
}

/// Strip trailing line terminators from a body string.
///
/// Parsed bodies include a trailing blank line when followed by a card fence
/// (required by the MARKDOWN.md §3 F2 rule); those characters are structural
/// separators, not part of what the document author wrote.
fn trim_body(body: &str) -> String {
    body.trim_end_matches(|c: char| c == '\n' || c == '\r')
        .to_string()
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
    if tree.is_instance_of::<js_sys::Map>() {
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
        return Ok(entries);
    }

    // Plain object: walk via `Object.entries`.
    if tree.is_object() && !tree.is_null() {
        let obj = tree.clone().unchecked_into::<js_sys::Object>();
        let pairs = js_sys::Object::entries(&obj);
        let mut entries: Vec<(String, JsValue)> = Vec::with_capacity(pairs.length() as usize);
        for i in 0..pairs.length() {
            let pair = Array::from(&pairs.get(i));
            let path = pair.get(0).as_string().ok_or_else(|| {
                WasmError::from("quill object key must be a string").to_js_value()
            })?;
            entries.push((path, pair.get(1)));
        }
        return Ok(entries);
    }

    Err(
        WasmError::from("quill requires a Map<string, Uint8Array> or Record<string, Uint8Array>")
            .to_js_value(),
    )
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
    pub fn render(&self, opts: Option<RenderOptions>) -> Result<RenderResult, JsValue> {
        let start = now_ms();
        let rust_opts: quillmark_core::RenderOptions = opts.unwrap_or_default().into();

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
