//! # AcroForm Backend for Quillmark
//!
//! This crate provides an AcroForm backend implementation that fills PDF form fields
//! with values rendered from YAML context using MiniJinja templates.
//!
//! ## Overview
//!
//! The primary entry point is the [`AcroformBackend`] struct, which implements the
//! [`Backend`] trait from `quillmark-core`. Instead of relying on quillmark-core's
//! templating for glue composition, this backend directly uses MiniJinja to render
//! form field values with Jinja-style templating expressions.
//!
//! ## Workflow
//!
//! 1. Read PDF form from quill's `form.pdf` file
//! 2. Extract field names and current values from the PDF form
//! 3. For each field, render the field value as a MiniJinja template with the JSON context
//! 4. Write the rendered values back to the PDF form
//! 5. Return the filled PDF as bytes
//!
//! ## Example Usage
//!
//! ```no_run
//! use quillmark_acroform::AcroformBackend;
//! use quillmark_core::{Backend, Quill, OutputFormat};
//!
//! let backend = AcroformBackend::default();
//! let quill = Quill::from_path("path/to/quill").unwrap();
//!
//! // Use with Workflow API (recommended)
//! // let workflow = Workflow::new(Box::new(backend), quill);
//! ```

use acroform::{AcroFormDocument, FieldValue};
use quillmark_core::{Artifact, Backend, Glue, OutputFormat, Quill, RenderError, RenderOptions};
use std::collections::HashMap;

/// AcroForm backend implementation for Quillmark.
pub struct AcroformBackend;

impl Backend for AcroformBackend {
    fn id(&self) -> &'static str {
        "acroform"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        &[OutputFormat::Pdf]
    }

    fn glue_type(&self) -> &'static str {
        ".json"
    }

    fn register_filters(&self, _glue: &mut Glue) {
        // No filters registered - we use default JSON glue
    }

    fn compile(
        &self,
        glue_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<Vec<Artifact>, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        // Check if format is supported
        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format,
            });
        }

        println!("AcroForm backend compiling for quill: {}", quill.name);

        // Parse the JSON context from glue_content
        let context: serde_json::Value = serde_json::from_str(glue_content).map_err(|e| {
            RenderError::Other(format!("Failed to parse JSON context: {}", e).into())
        })?;

        // Read form.pdf from the quill's file system
        let form_pdf_bytes = quill.files.get_file("form.pdf").ok_or_else(|| {
            RenderError::Other(format!("form.pdf not found in quill '{}'", quill.name).into())
        })?;

        // Load the PDF form directly from bytes (no temporary file needed)
        let mut doc = AcroFormDocument::from_bytes(form_pdf_bytes.to_vec())
            .map_err(|e| RenderError::Other(format!("Failed to load PDF form: {}", e).into()))?;

        // Create a MiniJinja environment for rendering field values
        let env = minijinja::Environment::new();

        // Get all form fields
        let fields = doc.fields().map_err(|e| {
            RenderError::Other(format!("Failed to get PDF form fields: {}", e).into())
        })?;

        // Prepare values to fill
        let mut values_to_fill = HashMap::new();

        for field in fields {
            // Get the current field value (which may contain a template)
            if let Some(field_value) = &field.current_value {
                let field_value_str = match field_value {
                    FieldValue::Text(s) => s.clone(),
                    FieldValue::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
                    FieldValue::Choice(s) => s.clone(),
                    FieldValue::Integer(i) => i.to_string(),
                };

                // Try to render the field value as a template
                match env.render_str(&field_value_str, &context) {
                    Ok(rendered_value) => {
                        // Only update if the rendered value is different from the original
                        if rendered_value != field_value_str {
                            values_to_fill
                                .insert(field.name.clone(), FieldValue::Text(rendered_value));
                        }
                    }
                    Err(_e) => {
                        // If rendering fails, keep the original value
                        // (it might not be a template)
                    }
                }
            }
        }

        // Fill the PDF form and get the result as bytes (in-memory)
        let output_bytes = doc
            .fill(values_to_fill)
            .map_err(|e| RenderError::Other(format!("Failed to fill PDF: {}", e).into()))?;

        Ok(vec![Artifact {
            bytes: output_bytes,
            output_format: OutputFormat::Pdf,
        }])
    }
}

impl Default for AcroformBackend {
    /// Creates a new [`AcroformBackend`] instance.
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_info() {
        let backend = AcroformBackend::default();
        assert_eq!(backend.id(), "acroform");
        assert_eq!(backend.glue_type(), ".json");
        assert!(backend.supported_formats().contains(&OutputFormat::Pdf));
    }
}
