//! AcroForm backend for Quillmark that fills PDF form fields with templated values.
//!
//! This backend reads PDF forms, renders field values using MiniJinja templates,
//! and returns filled PDFs. Fields can be templated via their current values or
//! via tooltip metadata in the format: `description__{{template}}`.

use acroform::{AcroFormDocument, FieldValue};
use quillmark_core::{
    Artifact, Backend, Glue, OutputFormat, Quill, RenderError, RenderOptions, RenderResult,
};
use std::collections::HashMap;

/// AcroForm backend implementation for Quillmark.
#[derive(Default)]
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
    ) -> Result<RenderResult, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format,
            });
        }
        let mut context: serde_json::Value = serde_json::from_str(glue_content).map_err(|e| {
            RenderError::Other(format!("Failed to parse JSON context: {}", e).into())
        })?;

        // Replace all null values with empty strings
        fn replace_nulls_with_empty(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Null => *value = serde_json::Value::String(String::new()),
                serde_json::Value::Object(map) => {
                    for v in map.values_mut() {
                        replace_nulls_with_empty(v);
                    }
                }
                serde_json::Value::Array(arr) => {
                    for v in arr.iter_mut() {
                        replace_nulls_with_empty(v);
                    }
                }
                _ => {}
            }
        }

        replace_nulls_with_empty(&mut context);

        let form_pdf_bytes = quill.files.get_file("form.pdf").ok_or_else(|| {
            RenderError::Other(format!("form.pdf not found in quill '{}'", quill.name).into())
        })?;

        let mut doc = AcroFormDocument::from_bytes(form_pdf_bytes.to_vec())
            .map_err(|e| RenderError::Other(format!("Failed to load PDF form: {}", e).into()))?;

        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

        let fields = doc.fields().map_err(|e| {
            RenderError::Other(format!("Failed to get PDF form fields: {}", e).into())
        })?;

        let mut values_to_fill = HashMap::new();

        for field in fields {
            // Extract template from tooltip (format: "description__{{template}}")
            let template_to_render = field.tooltip.as_ref().and_then(|tooltip| {
                tooltip.find("__").and_then(|pos| {
                    tooltip.get(pos + 2..).and_then(|template_part| {
                        if template_part.trim().is_empty() {
                            None
                        } else {
                            Some(template_part.to_string())
                        }
                    })
                })
            });

            let using_tooltip_template = template_to_render.is_some();

            // Determine what to render: tooltip template or field value
            let render_source = template_to_render.or_else(|| {
                field
                    .current_value
                    .as_ref()
                    .map(|field_value| match field_value {
                        FieldValue::Text(s) => s.clone(),
                        FieldValue::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
                        FieldValue::Choice(s) => s.clone(),
                        FieldValue::Integer(i) => i.to_string(),
                    })
            });

            if let Some(source) = render_source {
                if let Ok(rendered_value) = env.render_str(&source, &context) {
                    let should_update = using_tooltip_template || rendered_value != source;

                    if should_update {
                        let new_value = match &field.current_value {
                            Some(FieldValue::Text(_)) => FieldValue::Text(rendered_value),
                            Some(FieldValue::Boolean(_)) => {
                                let bool_val =
                                    rendered_value.trim().parse::<i32>().ok().map_or_else(
                                        || rendered_value.trim().to_lowercase() == "true",
                                        |num| num != 0,
                                    );
                                FieldValue::Boolean(bool_val)
                            }
                            Some(FieldValue::Choice(_)) => {
                                let choice_val = match rendered_value.trim().to_lowercase().as_str()
                                {
                                    "true" => "1".to_string(),
                                    "false" => "0".to_string(),
                                    _ => rendered_value,
                                };
                                FieldValue::Choice(choice_val)
                            }
                            Some(FieldValue::Integer(_)) => {
                                let int_val = match rendered_value.trim().to_lowercase().as_str() {
                                    "true" => 1,
                                    "false" => 0,
                                    _ => rendered_value.trim().parse::<i32>().unwrap_or(0),
                                };
                                FieldValue::Integer(int_val)
                            }
                            None => FieldValue::Text(rendered_value),
                        };
                        println!(
                            "Filling field '{}' with value '{:?}'\n",
                            field.name, new_value
                        );
                        values_to_fill.insert(field.name.clone(), new_value);
                    }
                }
            }
        }

        let output_bytes = doc
            .fill(values_to_fill)
            .map_err(|e| RenderError::Other(format!("Failed to fill PDF: {}", e).into()))?;

        let artifacts = vec![Artifact {
            bytes: output_bytes,
            output_format: OutputFormat::Pdf,
        }];

        Ok(RenderResult::new(artifacts, OutputFormat::Pdf))
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

    #[test]
    fn test_boolean_parsing() {
        // Test that boolean values are parsed correctly
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

        let context = serde_json::json!({
            "enabled": true,
            "disabled": false
        });

        // Test true
        let result = env.render_str("{{enabled}}", &context).unwrap();
        assert_eq!(result.trim().to_lowercase(), "true");

        // Test false
        let result = env.render_str("{{disabled}}", &context).unwrap();
        assert_eq!(result.trim().to_lowercase(), "false");
    }

    #[test]
    fn test_integer_parsing() {
        // Test that integer values are parsed correctly
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Chainable);

        let context = serde_json::json!({
            "count": 42,
            "negative": -10
        });

        // Test positive integer
        let result = env.render_str("{{count}}", &context).unwrap();
        assert_eq!(result.trim().parse::<i32>().unwrap(), 42);

        // Test negative integer
        let result = env.render_str("{{negative}}", &context).unwrap();
        assert_eq!(result.trim().parse::<i32>().unwrap(), -10);
    }
}
