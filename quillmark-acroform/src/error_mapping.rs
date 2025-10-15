//! Error mapping utilities for converting AcroForm errors to Quillmark diagnostics.

use quillmark_core::{Diagnostic, Location, RenderError, Severity, TemplateError};
use std::path::Path;

/// Map an AcroForm PdfError to a RenderError with a Diagnostic.
pub fn map_acroform_error(error: acroform::PdfError, form_path: &Path) -> RenderError {
    let message = format!("PDF form error: {}", error);

    let diagnostic = Diagnostic {
        severity: Severity::Error,
        code: None,
        message: message.clone(),
        primary: Some(Location {
            file: form_path.to_string_lossy().to_string(),
            line: 0,
            col: 0,
        }),
        related: vec![],
        hint: Some("Ensure the PDF file is a valid AcroForm document".to_string()),
    };

    RenderError::CompilationFailed(1, vec![diagnostic])
}

/// Map a serde_json parse error to a TemplateError
pub fn map_json_parse_error(error: serde_json::Error) -> TemplateError {
    TemplateError::InvalidTemplate(
        format!("Failed to parse JSON glue content: {}", error),
        Box::new(error),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_map_acroform_error() {
        let path = PathBuf::from("test.pdf");
        let error = acroform::PdfError::NotFound {
            word: "test".to_string(),
        };
        let result = map_acroform_error(error, &path);

        match result {
            RenderError::CompilationFailed(count, diagnostics) => {
                assert_eq!(count, 1);
                assert_eq!(diagnostics.len(), 1);
                let diagnostic = &diagnostics[0];
                assert!(diagnostic.message.contains("PDF form error"));
                assert_eq!(diagnostic.severity, Severity::Error);
                assert_eq!(diagnostic.primary.as_ref().unwrap().file, "test.pdf");
            }
            _ => panic!("Expected CompilationFailed"),
        }
    }
}
