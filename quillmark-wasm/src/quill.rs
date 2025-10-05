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

    /// Create Quill from in-memory file map (browser-friendly)
    #[wasm_bindgen(js_name = fromFiles)]
    pub fn from_files(files_js: JsValue, metadata_js: JsValue) -> Result<Quill, JsValue> {
        let _files: std::collections::HashMap<String, Vec<u8>> =
            serde_wasm_bindgen::from_value(files_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse files: {}", e)).to_js_value()
            })?;

        let _metadata: QuillMetadata =
            serde_wasm_bindgen::from_value(metadata_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse metadata: {}", e)).to_js_value()
            })?;

        Err(
            QuillmarkError::system("Quill.fromFiles is not yet fully implemented".to_string())
                .to_js_value(),
        )
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
