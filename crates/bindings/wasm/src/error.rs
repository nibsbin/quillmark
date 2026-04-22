//! Error handling utilities for WASM bindings

use quillmark_core::{Diagnostic, ParseError, RenderError, Severity};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Serializable error for JavaScript consumption.
///
/// Shape matches the success-path [`quillmark_core::Diagnostic`] so JS
/// consumers can use a single renderer for both thrown errors and warnings
/// in `RenderResult.warnings`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WasmError {
    /// Single diagnostic error
    Diagnostic {
        #[serde(flatten)]
        diagnostic: Diagnostic,
    },
    /// Multiple diagnostics (e.g., compilation errors)
    MultipleDiagnostics {
        message: String,
        diagnostics: Vec<Diagnostic>,
    },
}

impl WasmError {
    /// Convert to a JS `Error` object for throwing.
    ///
    /// Returns a real `Error` whose `.message` is the primary diagnostic
    /// message. Structured data is attached as a `.diagnostic` property for
    /// callers that need to branch on codes, severity, etc. The shape
    /// mirrors the diagnostics in `result.warnings`.
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
        WasmError::Diagnostic {
            diagnostic: error.to_diagnostic(),
        }
    }
}

impl From<RenderError> for WasmError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::CompilationFailed { diags } => WasmError::MultipleDiagnostics {
                message: format!("Compilation failed with {} error(s)", diags.len()),
                diagnostics: diags,
            },
            _ => {
                let diagnostic = error
                    .diagnostics()
                    .first()
                    .map(|d| (*d).clone())
                    .unwrap_or_else(|| Diagnostic::new(Severity::Error, error.to_string()));
                WasmError::Diagnostic { diagnostic }
            }
        }
    }
}

impl From<String> for WasmError {
    fn from(message: String) -> Self {
        WasmError::Diagnostic {
            diagnostic: Diagnostic::new(Severity::Error, message),
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
                assert_eq!(diagnostic.code.as_deref(), Some("parse::input_too_large"));
                assert!(diagnostic.message.contains("Input too large"));
            }
            _ => panic!("Expected Diagnostic variant"),
        }
    }
}
