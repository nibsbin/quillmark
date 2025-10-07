//! Quillmark WASM Engine - Simplified API

use crate::error::QuillmarkError;
use crate::types::{RenderOptions, RenderResult};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// Cross-platform helper to get current time in milliseconds as f64.
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
///
/// Create once, register Quills, render markdown. That's it.
#[wasm_bindgen]
pub struct Quillmark {
    inner: quillmark::Quillmark,
    quills: HashMap<String, quillmark_core::Quill>,
}

#[wasm_bindgen]
impl Quillmark {
    /// Create a new Quillmark engine
    #[wasm_bindgen]
    pub fn create() -> Quillmark {
        Quillmark {
            inner: quillmark::Quillmark::new(),
            quills: HashMap::new(),
        }
    }

    /// Register a Quill template bundle
    ///
    /// Accepts either a JSON string or a JsValue object representing the Quill file tree.
    /// Validation happens automatically on registration.
    #[wasm_bindgen(js_name = registerQuill)]
    pub fn register_quill(&mut self, name: &str, quill_json: JsValue) -> Result<(), JsValue> {
        // Convert JsValue to JSON string
        let json_str = if quill_json.is_string() {
            quill_json.as_string().ok_or_else(|| {
                QuillmarkError::new(
                    "Failed to convert JsValue to string".to_string(),
                    None,
                    None,
                )
                .to_js_value()
            })?
        } else {
            js_sys::JSON::stringify(&quill_json)
                .map_err(|e| {
                    QuillmarkError::new(
                        format!("Failed to serialize Quill JSON: {:?}", e),
                        None,
                        Some("Ensure the Quill object has the correct structure".to_string()),
                    )
                    .to_js_value()
                })?
                .as_string()
                .ok_or_else(|| {
                    QuillmarkError::new("Failed to convert JSON to string".to_string(), None, None)
                        .to_js_value()
                })?
        };

        // Parse and validate Quill
        let quill = quillmark_core::Quill::from_json(&json_str).map_err(|e| {
            QuillmarkError::new(
                format!("Failed to parse Quill: {}", e),
                None,
                Some("Ensure Quill.toml exists and is valid TOML".to_string()),
            )
            .to_js_value()
        })?;

        // Validate
        quill.validate().map_err(|e| {
            QuillmarkError::new(format!("Quill validation failed: {}", e), None, None).to_js_value()
        })?;

        // Register
        self.inner.register_quill(quill.clone());
        self.quills.insert(name.to_string(), quill);

        Ok(())
    }

    /// Process markdown through template engine (debugging)
    ///
    /// Returns template source code (Typst, LaTeX, etc.)
    #[wasm_bindgen(js_name = renderGlue)]
    pub fn render_glue(&mut self, quill_name: &str, markdown: &str) -> Result<String, JsValue> {
        let workflow = self.inner.load(quill_name).map_err(|e| {
            QuillmarkError::new(
                format!("Quill '{}' not found: {}", quill_name, e),
                None,
                Some("Use registerQuill() before rendering".to_string()),
            )
            .to_js_value()
        })?;

        workflow
            .process_glue(markdown)
            .map_err(|e| QuillmarkError::from(e).to_js_value())
    }

    /// Render markdown to final artifacts (PDF, SVG, TXT)
    #[wasm_bindgen]
    pub fn render(
        &mut self,
        quill_name: &str,
        markdown: &str,
        options: JsValue,
    ) -> Result<JsValue, JsValue> {
        let opts: RenderOptions = if options.is_undefined() || options.is_null() {
            RenderOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(|e| {
                QuillmarkError::new(
                    format!("Invalid render options: {}", e),
                    None,
                    Some("Check that format is 'pdf', 'svg', or 'txt'".to_string()),
                )
                .to_js_value()
            })?
        };

        let mut workflow = self.inner.load(quill_name).map_err(|e| {
            QuillmarkError::new(
                format!("Quill '{}' not found: {}", quill_name, e),
                None,
                Some("Use registerQuill() before rendering".to_string()),
            )
            .to_js_value()
        })?;

        // Add assets if provided
        if let Some(assets) = opts.assets {
            for (filename, bytes) in assets {
                workflow = workflow.with_asset(filename, bytes).map_err(|e| {
                    QuillmarkError::new(format!("Failed to add asset: {}", e), None, None)
                        .to_js_value()
                })?;
            }
        }

        let start = now_ms();

        let output_format = opts.format.map(|f| f.into());
        let result = workflow
            .render(markdown, output_format)
            .map_err(|e| QuillmarkError::from(e).to_js_value())?;

        let render_result = RenderResult {
            artifacts: result.artifacts.into_iter().map(Into::into).collect(),
            warnings: result.warnings.into_iter().map(Into::into).collect(),
            render_time_ms: now_ms() - start,
        };

        serde_wasm_bindgen::to_value(&render_result).map_err(|e| {
            QuillmarkError::new(format!("Failed to serialize result: {}", e), None, None)
                .to_js_value()
        })
    }

    /// List registered Quill names
    #[wasm_bindgen(js_name = listQuills)]
    pub fn list_quills(&self) -> Vec<String> {
        self.quills.keys().cloned().collect()
    }

    /// Unregister a Quill (free memory)
    #[wasm_bindgen(js_name = unregisterQuill)]
    pub fn unregister_quill(&mut self, name: &str) {
        self.quills.remove(name);
    }
}
