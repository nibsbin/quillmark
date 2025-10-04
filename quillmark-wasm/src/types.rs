//! Type definitions for the WASM API

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Output formats supported by backends
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    PDF,
    SVG,
    TXT,
}

impl From<OutputFormat> for quillmark_core::OutputFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::PDF => quillmark_core::OutputFormat::Pdf,
            OutputFormat::SVG => quillmark_core::OutputFormat::Svg,
            OutputFormat::TXT => quillmark_core::OutputFormat::Txt,
        }
    }
}

impl From<quillmark_core::OutputFormat> for OutputFormat {
    fn from(format: quillmark_core::OutputFormat) -> Self {
        match format {
            quillmark_core::OutputFormat::Pdf => OutputFormat::PDF,
            quillmark_core::OutputFormat::Svg => OutputFormat::SVG,
            quillmark_core::OutputFormat::Txt => OutputFormat::TXT,
        }
    }
}

/// Severity levels for diagnostics
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    ERROR,
    WARNING,
    NOTE,
}

impl From<quillmark_core::Severity> for Severity {
    fn from(severity: quillmark_core::Severity) -> Self {
        match severity {
            quillmark_core::Severity::Error => Severity::ERROR,
            quillmark_core::Severity::Warning => Severity::WARNING,
            quillmark_core::Severity::Note => Severity::NOTE,
        }
    }
}

/// Source location for errors and warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl From<quillmark_core::Location> for Location {
    fn from(loc: quillmark_core::Location) -> Self {
        Location {
            file: loc.file,
            line: loc.line as usize,
            column: loc.col as usize,
        }
    }
}

/// Diagnostic message (error, warning, or note)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(default)]
    pub related_locations: Vec<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl From<quillmark_core::Diagnostic> for Diagnostic {
    fn from(diag: quillmark_core::Diagnostic) -> Self {
        Diagnostic {
            severity: diag.severity.into(),
            code: None,
            message: diag.message,
            location: diag.primary.map(|loc| loc.into()),
            related_locations: diag.related.into_iter().map(|loc| loc.into()).collect(),
            hint: diag.hint,
        }
    }
}

/// Rendered artifact (PDF, SVG, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub format: OutputFormat,
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

impl Artifact {
    fn mime_type_for_format(format: OutputFormat) -> String {
        match format {
            OutputFormat::PDF => "application/pdf".to_string(),
            OutputFormat::SVG => "image/svg+xml".to_string(),
            OutputFormat::TXT => "text/plain".to_string(),
        }
    }
}

impl From<quillmark_core::Artifact> for Artifact {
    fn from(artifact: quillmark_core::Artifact) -> Self {
        let format = artifact.output_format.into();
        Artifact {
            format,
            mime_type: Self::mime_type_for_format(format),
            bytes: artifact.bytes,
        }
    }
}

/// Result of a render operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderResult {
    pub artifacts: Vec<Artifact>,
    pub warnings: Vec<Diagnostic>,
    pub metadata: RenderMetadata,
}

/// Metadata about the render operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMetadata {
    pub render_time_ms: f64,
    pub backend: String,
    pub quill_name: String,
}

/// Quill metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuillMetadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// Options for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
}

/// Engine creation options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_cache_size: Option<usize>,
}
