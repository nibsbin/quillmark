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
}

impl From<ParseError> for WasmError {
    fn from(error: ParseError) -> Self {
        match error {
            ParseError::MissingCardDirective { diag } => WasmError::Diagnostic {
                diagnostic: (*diag).into(),
            },
            ParseError::YamlError(e) => WasmError::Diagnostic {
                diagnostic: SerializableDiagnostic {
                    severity: quillmark_core::Severity::Error,
                    code: Some("yaml_error".to_string()),
                    message: format!("YAML parsing error: {}", e),
                    primary: None,
                    hint: None,
                    source_chain: vec![],
                },
            },
            ParseError::JsonError(e) => WasmError::Diagnostic {
                diagnostic: SerializableDiagnostic {
                    severity: quillmark_core::Severity::Error,
                    code: Some("json_error".to_string()),
                    message: format!("JSON conversion error: {}", e),
                    primary: None,
                    hint: None,
                    source_chain: vec![],
                },
            },
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
    use quillmark_core::{Diagnostic, Severity};

    #[test]
    fn test_missing_card_directive_conversion() {
        let diag = Diagnostic::new(Severity::Error, "Missing CARD".to_string())
            .with_code("parse::missing_card".to_string());

        let err = ParseError::MissingCardDirective {
            diag: Box::new(diag),
        };
        let wasm_err: WasmError = err.into();

        match wasm_err {
            WasmError::Diagnostic { diagnostic } => {
                assert_eq!(diagnostic.code.as_deref(), Some("parse::missing_card"));
                assert_eq!(diagnostic.message, "Missing CARD");
            }
            _ => panic!("Expected Diagnostic variant"),
        }
    }

    #[test]
    fn test_json_error_conversion() {
        // Create a JSON error (simulated)
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid-json").unwrap_err();
        let parse_err = ParseError::JsonError(json_err);
        let wasm_err: WasmError = parse_err.into();

        match wasm_err {
            WasmError::Diagnostic { diagnostic } => {
                assert_eq!(diagnostic.code.as_deref(), Some("json_error"));
                assert!(diagnostic.message.contains("JSON conversion error"));
            }
            _ => panic!("Expected Diagnostic variant"),
        }
    }
}
