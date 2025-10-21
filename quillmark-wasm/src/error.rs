//! Error types for the WASM API

use crate::types::{Diagnostic, Location};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Error type for Quillmark operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillmarkError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<Diagnostic>>,
}

impl QuillmarkError {
    pub fn new(message: String, location: Option<Location>, hint: Option<String>) -> Self {
        Self {
            message,
            location,
            hint,
            diagnostics: None,
        }
    }

    /// Convert to JsValue for throwing
    pub fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(self).unwrap_or_else(|_| JsValue::from_str(&self.message))
    }
}

impl From<quillmark_core::RenderError> for QuillmarkError {
    fn from(error: quillmark_core::RenderError) -> Self {
        use quillmark_core::RenderError;

        match error {
            RenderError::CompilationFailed { diags } => QuillmarkError {
                message: format!("Compilation failed with {} error(s)", diags.len()),
                location: None,
                hint: None,
                diagnostics: Some(diags.into_iter().map(|d| d.into()).collect()),
            },
            RenderError::TemplateFailed { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::InvalidFrontmatter { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::EngineCreation { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::FormatNotSupported { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::UnsupportedBackend { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::DynamicAssetCollision { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::DynamicFontCollision { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::InputTooLarge { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::YamlTooLarge { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::NestingTooDeep { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
            RenderError::OutputTooLarge { diag } => QuillmarkError {
                message: diag.message.clone(),
                location: diag.primary.map(|loc| loc.into()),
                hint: diag.hint.clone(),
                diagnostics: None,
            },
        }
    }
}

impl From<String> for QuillmarkError {
    fn from(message: String) -> Self {
        QuillmarkError::new(message, None, None)
    }
}

impl From<&str> for QuillmarkError {
    fn from(message: &str) -> Self {
        QuillmarkError::new(message.to_string(), None, None)
    }
}

impl std::fmt::Display for QuillmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for QuillmarkError {}
