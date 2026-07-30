//! The stamp spine's own error type.
//!
//! `quillmark-pdf` is leaf infra and owns a `PdfError` rather than threading
//! `quillmark_core::RenderError` through: every failure here carries just a
//! `code` and a `message`, so a struct (not a sprawling enum) is the honest
//! shape. The [`From`] impl carries one across a backend boundary, forwarding
//! the `pdf::*` code intact. Both backends share it, so the code's namespace is
//! decided in one place.

/// An error from the stamp spine. Carries a stable `code` (a `pdf::*` string a
/// consumer can match on) and a human-readable `message`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
#[non_exhaustive]
pub struct PdfError {
    /// Stable error code, e.g. `pdf::xref_stream`, `pdf::encrypted`.
    pub code: &'static str,
    /// Human-readable description.
    pub message: String,
}

impl PdfError {
    /// Build a `PdfError` from a code and message.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<PdfError> for quillmark_core::RenderError {
    fn from(e: PdfError) -> Self {
        quillmark_core::RenderError::from_diag(
            quillmark_core::Diagnostic::new(quillmark_core::Severity::Error, e.message)
                .with_code(e.code.to_string()),
        )
    }
}
