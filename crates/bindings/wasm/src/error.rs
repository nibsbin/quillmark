//! Error handling utilities for WASM bindings

use quillmark_core::{ParseError, RenderError, SerializableDiagnostic};
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

    /// Build a Diagnostic with an explicit error code.
    /// Use this instead of `WasmError::from(string)` when the call site knows
    /// a stable code that JS callers can branch on.
    pub fn with_code(code: &str, message: impl std::fmt::Display) -> Self {
        WasmError::Diagnostic {
            diagnostic: SerializableDiagnostic {
                severity: quillmark_core::Severity::Error,
                code: Some(code.to_string()),
                message: message.to_string(),
                primary: None,
                hint: None,
                source_chain: vec![],
            },
        }
    }
}

impl From<ParseError> for WasmError {
    fn from(error: ParseError) -> Self {
        match error {
            ParseError::InputTooLarge { size, max } => WasmError::Diagnostic {
                diagnostic: SerializableDiagnostic {
                    severity: quillmark_core::Severity::Error,
                    code: Some("input_too_large".to_string()),
                    message: format!("Input too large: {} bytes (max: {} bytes)", size, max),
                    primary: None,
                    hint: None,
                    source_chain: vec![],
                },
            },
            // Fallback for other errors to basic diagnostic
            _ => WasmError::Diagnostic {
                diagnostic: SerializableDiagnostic {
                    severity: quillmark_core::Severity::Error,
                    code: None,
                    message: error.to_string(),
                    primary: None,
                    hint: None,
                    source_chain: vec![],
                },
            },
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_too_large_conversion() {
        let err = ParseError::InputTooLarge {
            size: 1_000_000,
            max: 100_000,
        };
        let wasm_err: WasmError = err.into();

        match wasm_err {
            WasmError::Diagnostic { diagnostic } => {
                assert_eq!(diagnostic.code.as_deref(), Some("input_too_large"));
                assert!(diagnostic.message.contains("Input too large"));
            }
            _ => panic!("Expected Diagnostic variant"),
        }
    }
}
