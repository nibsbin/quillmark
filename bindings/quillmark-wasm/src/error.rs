//! Error handling utilities for WASM bindings

use quillmark_core::{RenderError, SerializableDiagnostic};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Serializable error for JavaScript consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum WasmError {
    /// Single diagnostic error
    Diagnostic {
        #[serde(flatten)]
        diagnostic: SerializableDiagnostic,
    },
    /// Multiple diagnostics (e.g., compilation errors)
    MultipleDiagnostics {
        message: String,
        diagnostics: Vec<SerializableDiagnostic>,
    },
}

impl WasmError {
    /// Convert to JsValue for throwing
    pub fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(self)
            .unwrap_or_else(|_| JsValue::from_str(&format!("{:?}", self)))
    }
}

impl From<RenderError> for WasmError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::CompilationFailed { diags } => WasmError::MultipleDiagnostics {
                message: format!("Compilation failed with {} error(s)", diags.len()),
                diagnostics: diags.into_iter().map(|d| d.into()).collect(),
            },
            // All other variants contain a single Diagnostic
            _ => {
                let diags = error.diagnostics();
                if let Some(diag) = diags.first() {
                    WasmError::Diagnostic {
                        diagnostic: (*diag).into(),
                    }
                } else {
                    // Fallback for edge cases
                    WasmError::Diagnostic {
                        diagnostic: SerializableDiagnostic {
                            severity: quillmark_core::Severity::Error,
                            code: None,
                            message: error.to_string(),
                            primary: None,
                            hint: None,
                            source_chain: vec![],
                        },
                    }
                }
            }
        }
    }
}

impl From<String> for WasmError {
    fn from(message: String) -> Self {
        WasmError::Diagnostic {
            diagnostic: SerializableDiagnostic {
                severity: quillmark_core::Severity::Error,
                code: None,
                message,
                primary: None,
                hint: None,
                source_chain: vec![],
            },
        }
    }
}

impl From<&str> for WasmError {
    fn from(message: &str) -> Self {
        WasmError::from(message.to_string())
    }
}
