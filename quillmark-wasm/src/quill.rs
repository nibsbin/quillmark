//! Quill class for managing template bundles

use crate::error::QuillmarkError;
use crate::types::QuillMetadata;
use wasm_bindgen::prelude::*;

/// Represents a Quill template bundle
#[wasm_bindgen]
pub struct Quill {
    inner: quillmark_core::Quill,
}

#[wasm_bindgen]
impl Quill {
    /// Create Quill from JSON string
    ///
    /// Accepts a JSON string representing a Quill folder structure.
    /// The JSON must follow the quillmark_core::Quill::from_json format:
    ///
    /// ```json
    /// {
    ///   "name": "optional-default-name",
    ///   "base_path": "/optional/base/path",
    ///   "files": {
    ///     "Quill.toml": { "contents": "...", "is_dir": false },
    ///     "glue.typ": { "contents": "...", "is_dir": false }
    ///   }
    /// }
    /// ```
    ///
    /// File contents can be either:
    /// - A UTF-8 string (recommended for text files)
    /// - An array of byte values (for binary files)
    ///
    /// The JSON should represent the entire Quill folder serialized.
    /// quillmark-core handles all parsing, ignoring, and validation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<Quill, JsValue> {
        let inner = quillmark_core::Quill::from_json(json_str).map_err(|e| {
            QuillmarkError::system(format!("Failed to create Quill from JSON: {}", e))
                .to_js_value()
        })?;

        Ok(Quill { inner })
    }

    /// Validate Quill structure (throws on error)
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner.validate().map_err(|e| {
            QuillmarkError::validation(format!("Quill validation failed: {}", e), vec![])
                .to_js_value()
        })
    }

    /// Get Quill metadata
    #[wasm_bindgen(js_name = getMetadata)]
    pub fn get_metadata(&self) -> Result<JsValue, JsValue> {
        let backend = self
            .inner
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let metadata = QuillMetadata {
            name: self.inner.name.clone(),
            version: None,
            backend,
            description: self
                .inner
                .metadata
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            author: self
                .inner
                .metadata
                .get("author")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        serde_wasm_bindgen::to_value(&metadata).map_err(|e| {
            QuillmarkError::system(format!("Failed to serialize metadata: {}", e)).to_js_value()
        })
    }

    /// List files in the Quill
    #[wasm_bindgen(js_name = listFiles)]
    pub fn list_files(&self) -> Vec<String> {
        self.inner
            .files
            .keys()
            .map(|path| path.to_string_lossy().to_string())
            .collect()
    }
}

impl Quill {
    /// Create a Quill from the internal representation
    pub(crate) fn from_inner(inner: quillmark_core::Quill) -> Self {
        Self { inner }
    }

    /// Take ownership of the internal Quill
    pub(crate) fn into_inner(self) -> quillmark_core::Quill {
        self.inner
    }
}
