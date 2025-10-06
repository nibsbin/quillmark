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
        use std::collections::HashMap;
        use std::path::PathBuf;

        let files: HashMap<String, Vec<u8>> =
            serde_wasm_bindgen::from_value(files_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse files: {}", e)).to_js_value()
            })?;

        let metadata_input: QuillMetadata =
            serde_wasm_bindgen::from_value(metadata_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse metadata: {}", e)).to_js_value()
            })?;

        // Parse Quill.toml
        let quill_toml_bytes = files.get("Quill.toml").ok_or_else(|| {
            QuillmarkError::system("Quill.toml not found in files".to_string()).to_js_value()
        })?;

        let quill_toml_content = String::from_utf8(quill_toml_bytes.clone()).map_err(|e| {
            QuillmarkError::system(format!("Quill.toml is not valid UTF-8: {}", e)).to_js_value()
        })?;

        let quill_toml: toml::Value = toml::from_str(&quill_toml_content).map_err(|e| {
            QuillmarkError::system(format!("Failed to parse Quill.toml: {}", e)).to_js_value()
        })?;

        // Extract fields from [Quill] section
        let mut metadata_map = HashMap::new();
        let mut glue_file = "glue.typ".to_string(); // default
        let mut template_file: Option<String> = None;
        let mut quill_name = metadata_input.name.clone();

        if let Some(quill_section) = quill_toml.get("Quill") {
            if let Some(name_val) = quill_section.get("name").and_then(|v| v.as_str()) {
                quill_name = name_val.to_string();
            }

            if let Some(backend_val) = quill_section.get("backend").and_then(|v| v.as_str()) {
                metadata_map.insert(
                    "backend".to_string(),
                    serde_yaml::Value::String(backend_val.to_string()),
                );
            } else {
                // Use backend from metadata input
                metadata_map.insert(
                    "backend".to_string(),
                    serde_yaml::Value::String(metadata_input.backend.clone()),
                );
            }

            if let Some(glue_val) = quill_section.get("glue").and_then(|v| v.as_str()) {
                glue_file = glue_val.to_string();
            }

            if let Some(template_val) = quill_section.get("template").and_then(|v| v.as_str()) {
                template_file = Some(template_val.to_string());
            }

            // Add other metadata fields
            if let Some(desc) = &metadata_input.description {
                metadata_map.insert(
                    "description".to_string(),
                    serde_yaml::Value::String(desc.clone()),
                );
            }

            if let Some(author) = &metadata_input.author {
                metadata_map.insert(
                    "author".to_string(),
                    serde_yaml::Value::String(author.clone()),
                );
            }
        }

        // Read glue template content
        let glue_template = files
            .get(&glue_file)
            .ok_or_else(|| {
                QuillmarkError::system(format!("Glue file '{}' not found", glue_file)).to_js_value()
            })
            .and_then(|bytes| {
                String::from_utf8(bytes.clone()).map_err(|e| {
                    QuillmarkError::system(format!(
                        "Glue file '{}' is not valid UTF-8: {}",
                        glue_file, e
                    ))
                    .to_js_value()
                })
            })?;

        // Read template content if specified
        let template_content = if let Some(ref template_file_name) = template_file {
            files
                .get(template_file_name)
                .and_then(|bytes| String::from_utf8(bytes.clone()).ok())
        } else {
            None
        };

        // Build file entries
        let mut file_entries = HashMap::new();
        for (path_str, bytes) in files {
            let path = PathBuf::from(&path_str);
            file_entries.insert(
                path.clone(),
                quillmark_core::FileEntry {
                    contents: bytes,
                    path: path.clone(),
                    is_dir: false,
                },
            );
        }

        // Create the Quill
        let inner = quillmark_core::Quill {
            glue_template,
            metadata: metadata_map,
            base_path: PathBuf::from("/"),
            name: quill_name,
            glue_file,
            template_file,
            template: template_content,
            files: file_entries,
        };

        // Validate the quill
        inner.validate().map_err(|e| {
            QuillmarkError::validation(format!("Quill validation failed: {}", e), vec![])
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
