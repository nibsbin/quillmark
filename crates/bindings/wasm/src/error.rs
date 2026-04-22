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
    /// Convert to a JS `Error` object for throwing.
    ///
    /// The returned value is a real `Error` instance whose `message` is the
    /// primary diagnostic message. Structured data is attached as a `diagnostic`
    /// property for callers that need to branch on error codes, severity, etc.
    ///
    /// Returning an `Error` (rather than a plain object or `Map`) ensures that
    /// JavaScript consumers — including Vitest's `toThrow(regex)` matcher —
    /// see `err instanceof Error === true` and `err.message` populated.
    pub fn to_js_value(&self) -> JsValue {
        let message = match self {
            WasmError::Diagnostic { diagnostic } => diagnostic.message.clone(),
            WasmError::MultipleDiagnostics { message, .. } => message.clone(),
        };
        let err = js_sys::Error::new(&message);
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        if let Ok(data) = self.serialize(&serializer) {
            let _ = js_sys::Reflect::set(&err, &JsValue::from_str("diagnostic"), &data);
        }
        err.into()
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
