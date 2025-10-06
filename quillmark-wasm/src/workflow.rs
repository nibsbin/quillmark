//! Workflow class for rendering documents

use crate::error::QuillmarkError;
use crate::types::{OutputFormat, RenderMetadata, RenderOptions, RenderResult};
use std::time::Instant;
use wasm_bindgen::prelude::*;

/// Rendering workflow for a specific Quill
#[wasm_bindgen]
pub struct Workflow {
    inner: quillmark::Workflow,
    quill_name: String,
    backend_id: String,
}

#[wasm_bindgen]
impl Workflow {
    /// Render markdown to artifacts
    pub fn render(&self, markdown: &str, options_js: JsValue) -> Result<JsValue, JsValue> {
        let start = Instant::now();

        // Parse options
        let options: RenderOptions = if options_js.is_undefined() || options_js.is_null() {
            RenderOptions { format: None }
        } else {
            serde_wasm_bindgen::from_value(options_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse render options: {}", e))
                    .to_js_value()
            })?
        };

        // Convert format if specified
        let output_format = options.format.map(|f| f.into());

        // Perform rendering
        let result = self
            .inner
            .render(markdown, output_format)
            .map_err(|e| QuillmarkError::from(e).to_js_value())?;

        let elapsed = start.elapsed();

        // Convert result
        let render_result = RenderResult {
            artifacts: result.artifacts.into_iter().map(|a| a.into()).collect(),
            warnings: result.warnings.into_iter().map(|d| d.into()).collect(),
            metadata: RenderMetadata {
                render_time_ms: elapsed.as_secs_f64() * 1000.0,
                backend: self.backend_id.clone(),
                quill_name: self.quill_name.clone(),
            },
        };

        serde_wasm_bindgen::to_value(&render_result).map_err(|e| {
            QuillmarkError::system(format!("Failed to serialize result: {}", e)).to_js_value()
        })
    }

    /// Render pre-processed glue content (advanced)
    #[wasm_bindgen(js_name = renderContent)]
    pub fn render_content(&self, content: &str, options_js: JsValue) -> Result<JsValue, JsValue> {
        let start = Instant::now();

        // Parse options
        let options: RenderOptions = if options_js.is_undefined() || options_js.is_null() {
            RenderOptions { format: None }
        } else {
            serde_wasm_bindgen::from_value(options_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse render options: {}", e))
                    .to_js_value()
            })?
        };

        // Convert format if specified
        let output_format = options.format.map(|f| f.into());

        // Perform rendering
        let result = self
            .inner
            .render_content(content, output_format)
            .map_err(|e| QuillmarkError::from(e).to_js_value())?;

        let elapsed = start.elapsed();

        // Convert result
        let render_result = RenderResult {
            artifacts: result.artifacts.into_iter().map(|a| a.into()).collect(),
            warnings: result.warnings.into_iter().map(|d| d.into()).collect(),
            metadata: RenderMetadata {
                render_time_ms: elapsed.as_secs_f64() * 1000.0,
                backend: self.backend_id.clone(),
                quill_name: self.quill_name.clone(),
            },
        };

        serde_wasm_bindgen::to_value(&render_result).map_err(|e| {
            QuillmarkError::system(format!("Failed to serialize result: {}", e)).to_js_value()
        })
    }

    /// Process markdown to glue without compilation (for debugging)
    #[wasm_bindgen(js_name = processGlue)]
    pub fn process_glue(&self, markdown: &str) -> Result<String, JsValue> {
        self.inner
            .process_glue(markdown)
            .map_err(|e| QuillmarkError::from(e).to_js_value())
    }

    /// Add dynamic asset (builder pattern)
    #[wasm_bindgen(js_name = withAsset)]
    pub fn with_asset(self, filename: String, bytes: Vec<u8>) -> Result<Workflow, JsValue> {
        let inner = self.inner.with_asset(filename, bytes).map_err(|e| {
            QuillmarkError::system(format!("Failed to add asset: {}", e)).to_js_value()
        })?;

        Ok(Workflow {
            inner,
            quill_name: self.quill_name,
            backend_id: self.backend_id,
        })
    }

    /// Add multiple dynamic assets
    #[wasm_bindgen(js_name = withAssets)]
    pub fn with_assets(self, assets_js: JsValue) -> Result<Workflow, JsValue> {
        let assets: std::collections::HashMap<String, Vec<u8>> =
            serde_wasm_bindgen::from_value(assets_js).map_err(|e| {
                QuillmarkError::system(format!("Failed to parse assets: {}", e)).to_js_value()
            })?;

        let mut inner = self.inner;
        for (filename, bytes) in assets {
            inner = inner.with_asset(filename, bytes).map_err(|e| {
                QuillmarkError::system(format!("Failed to add asset: {}", e)).to_js_value()
            })?;
        }

        Ok(Workflow {
            inner,
            quill_name: self.quill_name,
            backend_id: self.backend_id,
        })
    }

    /// Clear all dynamic assets
    #[wasm_bindgen(js_name = clearAssets)]
    pub fn clear_assets(self) -> Workflow {
        Workflow {
            inner: self.inner.clear_assets(),
            quill_name: self.quill_name,
            backend_id: self.backend_id,
        }
    }

    /// Get workflow metadata - backend ID
    #[wasm_bindgen(getter, js_name = backendId)]
    pub fn backend_id(&self) -> String {
        self.backend_id.clone()
    }

    /// Get workflow metadata - supported formats
    #[wasm_bindgen(getter, js_name = supportedFormats)]
    pub fn supported_formats(&self) -> Vec<OutputFormat> {
        self.inner
            .supported_formats()
            .iter()
            .map(|&f| f.into())
            .collect()
    }

    /// Get workflow metadata - quill name
    #[wasm_bindgen(getter, js_name = quillName)]
    pub fn quill_name(&self) -> String {
        self.quill_name.clone()
    }
}

impl Workflow {
    /// Create a new Workflow
    pub(crate) fn new(inner: quillmark::Workflow, quill_name: String, backend_id: String) -> Self {
        Self {
            inner,
            quill_name,
            backend_id,
        }
    }
}
