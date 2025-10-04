//! QuillmarkEngine class for managing Quills and rendering

use crate::error::QuillmarkError;
use crate::quill::Quill;
use crate::types::{EngineOptions, OutputFormat};
use crate::workflow::Workflow;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Main engine for managing Quills and rendering
#[wasm_bindgen]
pub struct QuillmarkEngine {
    inner: quillmark::Quillmark,
    quills: HashMap<String, quillmark_core::Quill>,
}

#[wasm_bindgen]
impl QuillmarkEngine {
    /// Create a new engine instance
    pub fn create(options_js: JsValue) -> Result<QuillmarkEngine, JsValue> {
        // Parse options if provided
        let _options: EngineOptions = if options_js.is_undefined() || options_js.is_null() {
            EngineOptions {
                enable_cache: None,
                max_cache_size: None,
            }
        } else {
            serde_wasm_bindgen::from_value(options_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse engine options: {}", e))
                    .to_js_value()
            })?
        };

        // Create engine with default backends
        let inner = quillmark::Quillmark::new();

        Ok(QuillmarkEngine {
            inner,
            quills: HashMap::new(),
        })
    }

    /// Register a Quill by name
    #[wasm_bindgen(js_name = registerQuill)]
    pub fn register_quill(&mut self, quill: Quill) -> Result<(), JsValue> {
        let inner_quill = quill.into_inner();
        let name = inner_quill.name.clone();

        self.inner.register_quill(inner_quill.clone());
        self.quills.insert(name, inner_quill);

        Ok(())
    }

    /// Unregister a Quill
    #[wasm_bindgen(js_name = unregisterQuill)]
    pub fn unregister_quill(&mut self, name: &str) {
        self.quills.remove(name);
    }

    /// List registered Quill names
    #[wasm_bindgen(js_name = listQuills)]
    pub fn list_quills(&self) -> Vec<String> {
        self.quills.keys().cloned().collect()
    }

    /// Get details about a registered Quill
    #[wasm_bindgen(js_name = getQuill)]
    pub fn get_quill(&self, name: &str) -> Option<Quill> {
        self.quills.get(name).map(|q| Quill::from_inner(q.clone()))
    }

    /// Load a workflow for rendering
    #[wasm_bindgen(js_name = loadWorkflow)]
    pub fn load_workflow(&mut self, quill_or_name: &str) -> Result<Workflow, JsValue> {
        // Try to load by name
        let workflow = self.inner.load(quill_or_name).map_err(|e| {
            QuillmarkError::system(format!("Failed to load workflow: {}", e)).to_js_value()
        })?;

        let backend_id = workflow.backend_id().to_string();

        Ok(Workflow::new(
            workflow,
            quill_or_name.to_string(),
            backend_id,
        ))
    }

    /// List available backends
    #[wasm_bindgen(js_name = listBackends)]
    pub fn list_backends(&self) -> Vec<String> {
        self.inner
            .registered_backends()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get supported formats for a backend
    #[wasm_bindgen(js_name = getSupportedFormats)]
    pub fn get_supported_formats(&self, _backend: &str) -> Result<Vec<OutputFormat>, JsValue> {
        // For now, return the formats for the default backend (typst)
        // This is a simplified implementation
        Ok(vec![
            OutputFormat::PDF,
            OutputFormat::SVG,
            OutputFormat::TXT,
        ])
    }

    /// Dispose of the engine and free resources
    pub fn dispose(&mut self) {
        self.quills.clear();
    }
}
