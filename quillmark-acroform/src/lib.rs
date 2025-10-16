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
//! 2. Extract field names, current values, and tooltips from the PDF form
//! 3. For each field:
//!    - If the field has a tooltip with template metadata (format: `description__{{template}}`),
//!      use the template part after `__` as the value to render
//!    - Otherwise, fall back to using the field's current value as a template
//! 4. Render the template with the JSON context using MiniJinja
//! 5. Write the rendered values back to the PDF form
//! 6. Return the filled PDF as bytes
//!
//! ## Tooltip Template Metadata
//!
//! The acroform library (v0.0.12+) extracts tooltips from PDF form fields. This backend
//! supports a special format for tooltips that includes template expressions:
//!
//! ```text
//! Description text__{{template.expression}}
//! ```
//!
//! The `__` (double underscore) separator splits the tooltip into:
//! - A human-readable description (before `__`)
//! - A MiniJinja template expression (after `__`)
//!
//! When a field has a tooltip with this format, the template expression is used to
//! determine the field's value, taking priority over the field's current value.
//!
//! ### Example
//!
//! If a PDF field has tooltip: `The name of the customer__{{customer.firstname}} {{customer.lastname}}`
//! and the JSON context contains:
//! ```json
//! {
//!   "customer": {
//!     "firstname": "John",
//!     "lastname": "Doe"
//!   }
//! }
//! ```
//! The field will be filled with: `John Doe`
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
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

        // Get all form fields
        let fields = doc.fields().map_err(|e| {
            RenderError::Other(format!("Failed to get PDF form fields: {}", e).into())
        })?;

        // Prepare values to fill
        let mut values_to_fill = HashMap::new();

        for field in fields {
            // Check if the field has a tooltip with template metadata
            let template_to_render = if let Some(tooltip) = &field.tooltip {
                // Check if tooltip contains "__" separator for template metadata
                if let Some(separator_pos) = tooltip.find("__") {
                    // Extract the template part after "__"
                    let template_part = &tooltip[separator_pos + 2..];
                    if !template_part.trim().is_empty() {
                        Some(template_part.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Determine what to render: tooltip template or field value
            let render_source = if let Some(template) = template_to_render {
                // Use the tooltip template
                Some(template)
            } else if let Some(field_value) = &field.current_value {
                // Fall back to the current field value (which may contain a template)
                let field_value_str = match field_value {
                    FieldValue::Text(s) => s.clone(),
                    FieldValue::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
                    FieldValue::Choice(s) => s.clone(),
                    FieldValue::Integer(i) => i.to_string(),
                };
                Some(field_value_str)
            } else {
                None
            };

            // Render the template if we have a source
            if let Some(source) = render_source {
                // Try to render the template
                match env.render_str(&source, &context) {
                    Ok(rendered_value) => {
                        // Always update with rendered value from tooltip template
                        // For field values, only update if different from original
                        let should_update = field.tooltip.is_some()
                            && field.tooltip.as_ref().unwrap().find("__").is_some()
                            || rendered_value != source;

                        if should_update {
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

    #[test]
    fn test_undefined_behavior_with_minijinja() {
        // Test that Chainable undefined behavior returns empty strings
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

        let context = serde_json::json!({
            "items": [
                {"name": "first"},
                {"name": "second"}
            ],
            "existing_key": "value"
        });

        // Test missing dictionary key
        let result = env.render_str("{{missing_key}}", &context);
        assert_eq!(
            result.unwrap(),
            "",
            "Missing key should render as empty string"
        );

        // Test out-of-bounds array access
        let result = env.render_str("{{items[10].name}}", &context);
        assert_eq!(
            result.unwrap(),
            "",
            "Out of bounds array access should render as empty string"
        );

        // Test nested missing property on undefined
        let result = env.render_str("{{items[10].name.nested}}", &context);
        assert_eq!(
            result.unwrap(),
            "",
            "Chained access on undefined should render as empty string"
        );

        // Test valid access still works
        let result = env.render_str("{{items[0].name}}", &context);
        assert_eq!(
            result.unwrap(),
            "first",
            "Valid access should work normally"
        );
    }
}
