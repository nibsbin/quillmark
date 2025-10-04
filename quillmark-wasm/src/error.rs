//! Error types for the WASM API

use crate::types::Diagnostic;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Error kind for categorizing errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorKind {
    Render,
    Validation,
    Network,
    System,
}

/// Error type for Quillmark operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillmarkError {
    pub kind: ErrorKind,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl QuillmarkError {
    pub fn new(kind: ErrorKind, message: String, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            kind,
            message,
            diagnostics,
        }
    }

    pub fn render(message: String, diagnostics: Vec<Diagnostic>) -> Self {
        Self::new(ErrorKind::Render, message, diagnostics)
    }

    pub fn validation(message: String, diagnostics: Vec<Diagnostic>) -> Self {
        Self::new(ErrorKind::Validation, message, diagnostics)
    }

    pub fn system(message: String) -> Self {
        Self::new(ErrorKind::System, message, vec![])
    }

    /// Convert to JsValue for throwing
    pub fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(self).unwrap_or_else(|_| JsValue::from_str(&self.message))
    }
}

impl From<quillmark_core::RenderError> for QuillmarkError {
    fn from(error: quillmark_core::RenderError) -> Self {
        use quillmark_core::RenderError;

        let (message, diagnostics) = match error {
            RenderError::CompilationFailed(count, diags) => (
                format!("Compilation failed with {} error(s)", count),
                diags.into_iter().map(|d| d.into()).collect(),
            ),
            RenderError::TemplateFailed { diag, .. } => (diag.message.clone(), vec![diag.into()]),
            RenderError::InvalidFrontmatter { diag, .. } => {
                (diag.message.clone(), vec![diag.into()])
            }
            RenderError::EngineCreation { diag, .. } => (diag.message.clone(), vec![diag.into()]),
            other => (other.to_string(), vec![]),
        };

        QuillmarkError::render(message, diagnostics)
    }
}

impl From<String> for QuillmarkError {
    fn from(message: String) -> Self {
        QuillmarkError::system(message)
    }
}

impl From<&str> for QuillmarkError {
    fn from(message: &str) -> Self {
        QuillmarkError::system(message.to_string())
    }
}

impl std::fmt::Display for QuillmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for QuillmarkError {}
