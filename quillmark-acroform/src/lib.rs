//! # AcroForm Backend for Quillmark
//!
//! This crate provides an AcroForm backend implementation that fills PDF form fields
//! using Jinja-style template expressions in field values.
//!
//! ## Overview
//!
//! The primary entry point is the [`AcroformBackend`] struct, which implements the
//! [`Backend`] trait from `quillmark-core`. Unlike template-based backends like Typst,
//! this backend uses the parsed document context to directly fill PDF form fields.
//!
//! ## Features
//!
//! - Loads PDF forms from a Quill's `form.pdf` file
//! - Extracts field names and current values
//! - Renders field values using MiniJinja templates with parsed document context
//! - Writes filled PDF forms as output artifacts
//!
//! ## Example Usage
//!
//! ```no_run
//! use quillmark_acroform::AcroformBackend;
//! use quillmark_core::{Backend, Quill};
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

mod error_mapping;
use error_mapping::map_acroform_error;

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
        // AcroForm backend doesn't use custom filters - it uses direct template rendering
        // No filters need to be registered
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

        // Parse the glue_content as JSON to get the document context
        let context: serde_json::Value = serde_json::from_str(glue_content)
            .map_err(|e| RenderError::Template(crate::error_mapping::map_json_parse_error(e)))?;

        // Load the PDF form from the quill
        let form_pdf_bytes = quill.files.get_file("form.pdf").ok_or_else(|| {
            RenderError::Internal(anyhow::anyhow!(
                "form.pdf not found in quill '{}'",
                quill.name
            ))
        })?;

        let bytes = fill_pdf_form(form_pdf_bytes, &context)?;

        Ok(vec![Artifact {
            bytes,
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

/// Fill a PDF form with values from the context using MiniJinja templates.
fn fill_pdf_form(
    form_pdf_bytes: &[u8],
    context: &serde_json::Value,
) -> Result<Vec<u8>, RenderError> {
    // Write the form bytes to a temporary file (acroform crate requires a file path)
    let temp_dir = std::env::temp_dir();
    let temp_form_path = temp_dir.join(format!("form_{}.pdf", uuid()));
    std::fs::write(&temp_form_path, form_pdf_bytes)
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("Failed to write temp form: {}", e)))?;

    // Load the PDF form
    let mut doc = AcroFormDocument::from_pdf(&temp_form_path)
        .map_err(|e| map_acroform_error(e, &temp_form_path))?;

    // Get all form fields
    let fields = doc
        .fields()
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("Failed to read form fields: {}", e)))?;

    // Prepare a HashMap to store the filled values
    let mut filled_values = HashMap::new();

    // Iterate through each field and render its value if it contains a template
    for field in fields {
        let field_name = field.name.clone();

        // Get the current value of the field (which should contain a Jinja template)
        if let Some(current_value) = field.current_value {
            match current_value {
                FieldValue::Text(template_str) => {
                    // Render the template with the context
                    let rendered = render_template(&template_str, context)?;

                    if !rendered.is_empty() {
                        filled_values.insert(field_name, FieldValue::Text(rendered));
                    }
                }
                FieldValue::Boolean(val) => {
                    // For boolean fields, keep as-is unless there's a special template
                    filled_values.insert(field_name, FieldValue::Boolean(val));
                }
                FieldValue::Choice(template_str) => {
                    // Render choice fields similarly
                    let rendered = render_template(&template_str, context)?;

                    if !rendered.is_empty() {
                        filled_values.insert(field_name, FieldValue::Choice(rendered));
                    }
                }
                FieldValue::Integer(val) => {
                    // For integer fields, keep as-is
                    filled_values.insert(field_name, FieldValue::Integer(val));
                }
            }
        }
    }

    // Fill the form and save to a byte vector
    let output_path = std::env::temp_dir().join(format!("filled_form_{}.pdf", uuid()));
    doc.fill_and_save(filled_values, &output_path)
        .map_err(|e| {
            RenderError::Internal(anyhow::anyhow!("Failed to fill and save PDF form: {}", e))
        })?;

    // Read the filled PDF into a byte vector
    let bytes = std::fs::read(&output_path)
        .map_err(|e| RenderError::Internal(anyhow::anyhow!("Failed to read filled PDF: {}", e)))?;

    // Clean up the temporary files
    let _ = std::fs::remove_file(&output_path);
    let _ = std::fs::remove_file(&temp_form_path);

    Ok(bytes)
}

/// Render a template string with the given context.
fn render_template(template_str: &str, context: &serde_json::Value) -> Result<String, RenderError> {
    // Create a new environment for each render to avoid lifetime issues
    let mut env = minijinja::Environment::new();

    // Add the template to the environment
    env.add_template("field", template_str)
        .map_err(|e| RenderError::from(e))?;

    // Render the template with the context
    let tmpl = env
        .get_template("field")
        .map_err(|e| RenderError::from(e))?;

    tmpl.render(context).map_err(|e| RenderError::from(e))
}

/// Generate a simple unique identifier for temporary files.
fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}", timestamp)
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
        assert!(!backend.supported_formats().contains(&OutputFormat::Svg));
    }

    #[test]
    fn test_render_template() {
        let context = serde_json::json!({"test": "success!"});
        let template = "{{ test }}";

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "success!");
    }

    #[test]
    fn test_render_template_with_object() {
        let context = serde_json::json!({"user": {"name": "Alice"}});
        let template = "Hello {{ user.name }}";

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "Hello Alice");
    }
}
