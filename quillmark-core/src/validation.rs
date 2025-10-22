//! Schema validation module for Quillmark.
//!
//! This module provides utilities for converting TOML field definitions to JSON Schema
//! and validating ParsedDocument data against schemas.

use crate::{quill::FieldSchema, QuillValue, RenderError};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// Convert a HashMap of FieldSchema to a JSON Schema object
pub fn build_schema_from_fields(
    field_schemas: &HashMap<String, FieldSchema>,
) -> Result<QuillValue, RenderError> {
    let mut properties = Map::new();
    let mut required_fields = Vec::new();

    for (field_name, field_schema) in field_schemas {
        // Build property schema
        let mut property = Map::new();

        // Add name
        property.insert("name".to_string(), Value::String(field_schema.name.clone()));

        // Add type if specified
        if let Some(ref field_type) = field_schema.r#type {
            let json_type = match field_type.as_str() {
                "str" => "string",
                "number" => "number",
                "array" => "array",
                "dict" => "object",
                "date" => "string",
                "datetime" => "string",
                _ => "string", // default to string for unknown types
            };
            property.insert("type".to_string(), Value::String(json_type.to_string()));

            // Add format for date types
            if field_type == "date" {
                property.insert("format".to_string(), Value::String("date".to_string()));
            } else if field_type == "datetime" {
                property.insert("format".to_string(), Value::String("date-time".to_string()));
            }
        }

        // Add description
        property.insert(
            "description".to_string(),
            Value::String(field_schema.description.clone()),
        );

        properties.insert(field_name.clone(), Value::Object(property));

        // Determine if field is required based on the spec:
        // - If default is present → field is optional
        // - If default is absent and required is true → field is required
        // - If default is absent and required is false → field is optional
        if field_schema.default.is_none() && field_schema.default.is_none() {
            required_fields.push(field_name.clone());
        }
    }

    // Build the complete JSON Schema
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2019-09/schema",
        "type": "object",
        "properties": properties,
        "required": required_fields,
        "additionalProperties": true
    });

    Ok(QuillValue::from_json(schema))
}

/// Validate a document's fields against a JSON Schema
pub fn validate_document(
    schema: &QuillValue,
    fields: &HashMap<String, crate::value::QuillValue>,
) -> Result<(), Vec<String>> {
    // Convert fields to JSON Value for validation
    let mut doc_json = Map::new();
    for (key, value) in fields {
        doc_json.insert(key.clone(), value.as_json().clone());
    }
    let doc_value = Value::Object(doc_json);

    // Compile the schema
    let compiled = match jsonschema::Validator::new(schema.as_json()) {
        Ok(c) => c,
        Err(e) => return Err(vec![format!("Failed to compile schema: {}", e)]),
    };

    // Validate the document and collect errors immediately
    let validation_result = compiled.validate(&doc_value);

    match validation_result {
        Ok(_) => Ok(()),
        Err(error) => {
            let path = error.instance_path.to_string();
            let path_display = if path.is_empty() {
                "document".to_string()
            } else {
                path
            };
            let message = format!("Validation error at {}: {}", path_display, error);
            Err(vec![message])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quill::FieldSchema;
    use crate::value::QuillValue;

    #[test]
    fn test_build_schema_simple() {
        let mut fields = HashMap::new();
        let mut schema = FieldSchema::new(
            "Author name".to_string(),
            "The name of the author".to_string(),
        );
        schema.r#type = Some("str".to_string());
        fields.insert("author".to_string(), schema);

        let json_schema = build_schema_from_fields(&fields).unwrap().as_json().clone();
        assert_eq!(json_schema["type"], "object");
        assert_eq!(json_schema["properties"]["author"]["type"], "string");
        assert_eq!(json_schema["properties"]["author"]["name"], "Author name");
        assert_eq!(
            json_schema["properties"]["author"]["description"],
            "The name of the author"
        );
    }

    #[test]
    fn test_build_schema_with_default() {
        let mut fields = HashMap::new();
        let mut schema = FieldSchema::new(
            "Field with default".to_string(),
            "A field with a default value".to_string(),
        );
        schema.r#type = Some("str".to_string());
        schema.default = Some(QuillValue::from_json(json!("default value")));
        // When default is present, field should be optional regardless of required flag
        fields.insert("with_default".to_string(), schema);

        build_schema_from_fields(&fields).unwrap();
    }

    #[test]
    fn test_build_schema_date_types() {
        let mut fields = HashMap::new();

        let mut date_schema =
            FieldSchema::new("Date field".to_string(), "A field for dates".to_string());
        date_schema.r#type = Some("date".to_string());
        fields.insert("date_field".to_string(), date_schema);

        let mut datetime_schema = FieldSchema::new(
            "DateTime field".to_string(),
            "A field for date and time".to_string(),
        );
        datetime_schema.r#type = Some("datetime".to_string());
        fields.insert("datetime_field".to_string(), datetime_schema);

        let json_schema = build_schema_from_fields(&fields).unwrap().as_json().clone();
        assert_eq!(json_schema["properties"]["date_field"]["type"], "string");
        assert_eq!(json_schema["properties"]["date_field"]["format"], "date");
        assert_eq!(
            json_schema["properties"]["datetime_field"]["type"],
            "string"
        );
        assert_eq!(
            json_schema["properties"]["datetime_field"]["format"],
            "date-time"
        );
    }

    #[test]
    fn test_validate_document_success() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "count": {"type": "number"}
            },
            "required": ["title"],
            "additionalProperties": true
        });

        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(json!("Test Title")),
        );
        fields.insert("count".to_string(), QuillValue::from_json(json!(42)));

        let result = validate_document(&QuillValue::from_json(schema), &fields);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_document_missing_required() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {"type": "string"}
            },
            "required": ["title"],
            "additionalProperties": true
        });

        let fields = HashMap::new(); // empty, missing required field

        let result = validate_document(&QuillValue::from_json(schema), &fields);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_validate_document_wrong_type() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "count": {"type": "number"}
            },
            "additionalProperties": true
        });

        let mut fields = HashMap::new();
        fields.insert(
            "count".to_string(),
            QuillValue::from_json(json!("not a number")),
        );

        let result = validate_document(&QuillValue::from_json(schema), &fields);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_document_allows_extra_fields() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {"type": "string"}
            },
            "required": ["title"],
            "additionalProperties": true
        });

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), QuillValue::from_json(json!("Test")));
        fields.insert("extra".to_string(), QuillValue::from_json(json!("allowed")));

        let result = validate_document(&QuillValue::from_json(schema), &fields);
        assert!(result.is_ok());
    }
}
