//! Quill class for managing template bundles

use crate::error::QuillmarkError;
use crate::types::QuillMetadata;
use std::path::{Path, PathBuf};
use wasm_bindgen::prelude::*;

/// Represents a Quill template bundle
#[wasm_bindgen]
pub struct Quill {
    inner: quillmark_core::Quill,
}

#[wasm_bindgen]
impl Quill {
    /// Create Quill from JSON
    ///
    /// Accepts a JSON string describing the Quill file tree. See the canonical
    /// contract at `designs/QUILL_DESIGN.md` for the precise shape and examples.
    /// The WASM wrapper exposes this as `Quill.fromJson()` (JS) which forwards
    /// to `quillmark_core::Quill::from_json`.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<Quill, JsValue> {
        let inner = quillmark_core::Quill::from_json(json_str).map_err(|e| {
            QuillmarkError::new(
                format!("Failed to create Quill from JSON: {}", e),
                None,
                None,
            )
            .to_js_value()
        })?;

        Ok(Quill { inner })
    }

    /// Create Quill from files object
    ///
    /// Accepts a JS object describing the Quill file tree. Internally converts
    /// to JSON and calls from_json. The object should have the structure:
    /// ```js
    /// {
    ///   metadata: { name: "my-quill", ... },  // optional
    ///   files: {
    ///     "Quill.toml": { contents: "..." },
    ///     "glue.typ": { contents: "..." },
    ///     ...
    ///   }
    /// }
    /// ```
    #[wasm_bindgen(js_name = fromFiles)]
    pub fn from_files(files_obj: JsValue) -> Result<Quill, JsValue> {
        // Convert JS object to JSON string
        let json_str = js_sys::JSON::stringify(&files_obj)
            .map_err(|e| {
                QuillmarkError::new(
                    format!("Failed to stringify files object: {:?}", e),
                    None,
                    None,
                )
                .to_js_value()
            })?
            .as_string()
            .ok_or_else(|| {
                QuillmarkError::new("Failed to convert JSON to string".to_string(), None, None)
                    .to_js_value()
            })?;

        // Call from_json with the stringified object
        Self::from_json(&json_str)
    }

    /// Validate Quill structure (throws on error)
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner.validate().map_err(|e| {
            QuillmarkError::new(format!("Quill validation failed: {}", e), None, None).to_js_value()
        })
    }

    /// Get Quill metadata
    #[wasm_bindgen(js_name = getMetadata)]
    pub fn get_metadata(&self) -> Result<JsValue, JsValue> {
        let metadata = QuillMetadata {
            name: self.inner.name.clone(),
            version: self
                .inner
                .metadata
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            backend: self
                .inner
                .metadata
                .get("backend")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
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
            license: self
                .inner
                .metadata
                .get("license")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tags: self
                .inner
                .metadata
                .get("tags")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
        };

        serde_wasm_bindgen::to_value(&metadata).map_err(|e| {
            QuillmarkError::new(format!("Failed to serialize metadata: {}", e), None, None)
                .to_js_value()
        })
    }

    /// Get field schemas as a JS object
    #[wasm_bindgen(js_name = getFieldSchemas)]
    pub fn get_field_schemas(&self) -> Result<JsValue, JsValue> {
        // Convert the field_schemas HashMap to a JS object
        let js_obj = js_sys::Object::new();

        for (field_name, schema_value) in &self.inner.field_schemas {
            // Convert serde_yaml::Value to serde_json::Value for serialization
            let json_value = serde_json::to_value(schema_value).map_err(|e| {
                QuillmarkError::new(
                    format!("Failed to convert field schema '{}': {}", field_name, e),
                    None,
                    None,
                )
                .to_js_value()
            })?;

            let js_value = serde_wasm_bindgen::to_value(&json_value).map_err(|e| {
                QuillmarkError::new(
                    format!("Failed to serialize field schema '{}': {}", field_name, e),
                    None,
                    None,
                )
                .to_js_value()
            })?;

            js_sys::Reflect::set(&js_obj, &JsValue::from_str(field_name), &js_value).map_err(
                |e| {
                    QuillmarkError::new(
                        format!("Failed to set field schema '{}': {:?}", field_name, e),
                        None,
                        None,
                    )
                    .to_js_value()
                },
            )?;
        }

        Ok(js_obj.into())
    }

    /// List all files in the Quill (recursive paths)
    #[wasm_bindgen(js_name = listFiles)]
    pub fn list_files(&self) -> Vec<String> {
        let mut all_files = Vec::new();
        Self::collect_all_file_paths(&self.inner.files, Path::new(""), &mut all_files);
        all_files
    }

    /// Check if a file exists
    #[wasm_bindgen(js_name = fileExists)]
    pub fn file_exists(&self, path: &str) -> bool {
        self.inner.file_exists(path)
    }

    /// Get file contents as Uint8Array
    #[wasm_bindgen(js_name = getFile)]
    pub fn get_file(&self, path: &str) -> Option<Vec<u8>> {
        self.inner.get_file(path).map(|bytes| bytes.to_vec())
    }

    /// Get file contents as string (UTF-8)
    #[wasm_bindgen(js_name = getFileAsString)]
    pub fn get_file_as_string(&self, path: &str) -> Option<String> {
        self.inner
            .get_file(path)
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }

    /// Check if a directory exists
    #[wasm_bindgen(js_name = dirExists)]
    pub fn dir_exists(&self, path: &str) -> bool {
        self.inner.dir_exists(path)
    }
}

impl Quill {
    /// Helper to recursively collect all file paths from tree
    fn collect_all_file_paths(
        node: &quillmark_core::FileTreeNode,
        current_path: &Path,
        result: &mut Vec<String>,
    ) {
        use quillmark_core::FileTreeNode;

        match node {
            FileTreeNode::File { .. } => {
                if current_path != Path::new("") {
                    result.push(current_path.to_string_lossy().to_string());
                }
            }
            FileTreeNode::Directory { files } => {
                for (name, child_node) in files {
                    let child_path = if current_path == Path::new("") {
                        PathBuf::from(name)
                    } else {
                        current_path.join(name)
                    };
                    Self::collect_all_file_paths(child_node, &child_path, result);
                }
            }
        }
    }

    // /// Create a Quill from the internal representation
    // pub(crate) fn from_inner(inner: quillmark_core::Quill) -> Self {
    //     Self { inner }
    // }

    // /// Take ownership of the internal Quill
    // pub(crate) fn into_inner(self) -> quillmark_core::Quill {
    //     self.inner
    // }
}
