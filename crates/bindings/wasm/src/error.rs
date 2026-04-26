//! Error handling utilities for WASM bindings

use crate::types::Diagnostic as WasmDiagnostic;
use quillmark_core::{Diagnostic, ParseError, RenderError, Severity};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Serializable error for JavaScript consumption.
///
/// Single uniform shape regardless of underlying error variant:
///
/// ```text
/// { message: string, diagnostics: Diagnostic[] }
/// ```
///
/// `diagnostics` is always a non-empty array — length 1 for
/// single-diagnostic errors, length N for compilation failures. The thrown
/// JS `Error` has its `.message` set to `message` and a `.diagnostics`
/// property attached carrying the array. Read `err.diagnostics[0]` for the
/// primary diagnostic.
#[derive(Debug, Clone)]
pub struct WasmError {
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl WasmError {
    /// Convert to a JS `Error` object for throwing.
    ///
    /// Returns a real `Error` whose `.message` is `self.message` and whose
    /// `.diagnostics` property is an array of diagnostic objects matching
    /// the shape used in `RenderResult.warnings`.
    pub fn to_js_value(&self) -> JsValue {
        let err = js_sys::Error::new(&self.message);
        let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        let wasm_diags: Vec<WasmDiagnostic> =
            self.diagnostics.iter().cloned().map(Into::into).collect();
        if let Ok(data) = wasm_diags.serialize(&serializer) {
            let _ = js_sys::Reflect::set(&err, &JsValue::from_str("diagnostics"), &data);
        }
        err.into()
    }
}

impl From<ParseError> for WasmError {
    fn from(error: ParseError) -> Self {
        let diag = error.to_diagnostic();
        WasmError {
            message: diag.message.clone(),
            diagnostics: vec![diag],
        }
    }
}

impl From<RenderError> for WasmError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::CompilationFailed { diags } => WasmError {
                message: format!("Compilation failed with {} error(s)", diags.len()),
                diagnostics: diags,
            },
            _ => {
                let diagnostic = error
                    .diagnostics()
                    .first()
                    .map(|d| (*d).clone())
                    .unwrap_or_else(|| Diagnostic::new(Severity::Error, error.to_string()));
                WasmError {
                    message: diagnostic.message.clone(),
                    diagnostics: vec![diagnostic],
                }
            }
        }
    }
}

impl From<String> for WasmError {
    fn from(message: String) -> Self {
        WasmError {
            message: message.clone(),
            diagnostics: vec![Diagnostic::new(Severity::Error, message)],
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

        assert_eq!(wasm_err.diagnostics.len(), 1);
        let diag = &wasm_err.diagnostics[0];
        assert_eq!(diag.code.as_deref(), Some("parse::input_too_large"));
        assert!(diag.message.contains("Input too large"));
        assert_eq!(wasm_err.message, diag.message);
    }

    #[test]
    fn test_compilation_failed_carries_all_diagnostics() {
        let diag1 = Diagnostic::new(Severity::Error, "Error 1".to_string());
        let diag2 = Diagnostic::new(Severity::Error, "Error 2".to_string());
        let render_err = RenderError::CompilationFailed {
            diags: vec![diag1, diag2],
        };
        let wasm_err: WasmError = render_err.into();

        assert_eq!(wasm_err.diagnostics.len(), 2);
        assert_eq!(wasm_err.diagnostics[0].message, "Error 1");
        assert_eq!(wasm_err.diagnostics[1].message, "Error 2");
        assert!(wasm_err.message.contains("2"));
    }

    #[test]
    fn test_string_conversion_yields_single_diagnostic() {
        let wasm_err: WasmError = "Simple error".into();
        assert_eq!(wasm_err.message, "Simple error");
        assert_eq!(wasm_err.diagnostics.len(), 1);
        assert_eq!(wasm_err.diagnostics[0].message, "Simple error");
    }
}
