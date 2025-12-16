//! Schema validation and utilities for Quillmark.
//!
//! This module provides utilities for converting TOML field definitions to JSON Schema
//! and validating ParsedDocument data against schemas.

use crate::{quill::FieldSchema, QuillValue, RenderError};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// Build a single field property JSON Schema object from a FieldSchema
fn build_field_property(field_schema: &FieldSchema) -> Map<String, Value> {
    let mut property = Map::new();

    // Add name
    property.insert("name".to_string(), Value::String(field_schema.name.clone()));

    // Handle scope type specially - generates array with items object
    if field_schema.r#type.as_deref() == Some("scope") {
        property.insert("type".to_string(), Value::String("array".to_string()));

        // Build items schema for scope
        let mut items_schema = Map::new();
        items_schema.insert("type".to_string(), Value::String("object".to_string()));

        if let Some(ref items) = field_schema.items {
            let mut item_properties = Map::new();
            let mut item_required = Vec::new();

            for (item_name, item_schema) in items {
                // Recursively build property for each item field
                let item_property = build_field_property(item_schema);
                item_properties.insert(item_name.clone(), Value::Object(item_property));

                // Item fields are required if explicitly marked as required = true
                if item_schema.required {
                    item_required.push(Value::String(item_name.clone()));
                }
            }

            items_schema.insert("properties".to_string(), Value::Object(item_properties));
            if !item_required.is_empty() {
                items_schema.insert("required".to_string(), Value::Array(item_required));
            }
            items_schema.insert("additionalProperties".to_string(), Value::Bool(true));
        }

        property.insert("items".to_string(), Value::Object(items_schema));
    } else if let Some(ref field_type) = field_schema.r#type {
        // Regular type handling
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

    // Add title if specified
    if let Some(ref title) = field_schema.title {
        property.insert("title".to_string(), Value::String(title.clone()));
    }

    // Add description
    property.insert(
        "description".to_string(),
        Value::String(field_schema.description.clone()),
    );

    // Add UI metadata as x-ui property if present
    if let Some(ref ui) = field_schema.ui {
        let mut ui_obj = Map::new();

        if let Some(ref group) = ui.group {
            ui_obj.insert("group".to_string(), Value::String(group.clone()));
        }

        if let Some(order) = ui.order {
            ui_obj.insert("order".to_string(), json!(order));
        }

        if !ui_obj.is_empty() {
            property.insert("x-ui".to_string(), Value::Object(ui_obj));
        }
    }

    // Add examples if specified
    if let Some(ref examples) = field_schema.examples {
        if let Some(examples_array) = examples.as_array() {
            if !examples_array.is_empty() {
                property.insert("examples".to_string(), Value::Array(examples_array.clone()));
            }
        }
    }

    // Add default if specified
    if let Some(ref default) = field_schema.default {
        property.insert("default".to_string(), default.as_json().clone());
    }

    property
}

/// Convert a HashMap of FieldSchema to a JSON Schema object
pub fn build_schema_from_fields(
    field_schemas: &HashMap<String, FieldSchema>,
) -> Result<QuillValue, RenderError> {
    let mut properties = Map::new();
    let mut required_fields = Vec::new();

    for (field_name, field_schema) in field_schemas {
        let property = build_field_property(field_schema);
        properties.insert(field_name.clone(), Value::Object(property));

        // Field is required if explicitly marked as required = true
        // Fields are optional by default (JSON Schema standard)
        if field_schema.required {
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

/// Extract default values from a JSON Schema
///
/// Parses the JSON schema's "properties" object and extracts any "default" values
/// defined for each property. Returns a HashMap mapping field names to their default
/// values.
///
/// # Arguments
///
/// * `schema` - A JSON Schema object (must have "properties" field)
///
/// # Returns
///
/// A HashMap of field names to their default QuillValues
pub fn extract_defaults_from_schema(
    schema: &QuillValue,
) -> HashMap<String, crate::value::QuillValue> {
    let mut defaults = HashMap::new();

    // Get the properties object from the schema
    if let Some(properties) = schema.as_json().get("properties") {
        if let Some(properties_obj) = properties.as_object() {
            for (field_name, field_schema) in properties_obj {
                // Check if this field has a default value
                if let Some(default_value) = field_schema.get("default") {
                    defaults.insert(
                        field_name.clone(),
                        QuillValue::from_json(default_value.clone()),
                    );
                }
            }
        }
    }

    defaults
}

/// Extract example values from a JSON Schema
///
/// Parses the JSON schema's "properties" object and extracts any "examples" arrays
/// defined for each property. Returns a HashMap mapping field names to their examples
/// (as an array of QuillValues).
///
/// # Arguments
///
/// * `schema` - A JSON Schema object (must have "properties" field)
///
/// # Returns
///
/// A HashMap of field names to their examples (``Vec<QuillValue>``)
pub fn extract_examples_from_schema(
    schema: &QuillValue,
) -> HashMap<String, Vec<crate::value::QuillValue>> {
    let mut examples = HashMap::new();

    // Get the properties object from the schema
    if let Some(properties) = schema.as_json().get("properties") {
        if let Some(properties_obj) = properties.as_object() {
            for (field_name, field_schema) in properties_obj {
                // Check if this field has examples
                if let Some(examples_value) = field_schema.get("examples") {
                    if let Some(examples_array) = examples_value.as_array() {
                        let examples_vec: Vec<QuillValue> = examples_array
                            .iter()
                            .map(|v| QuillValue::from_json(v.clone()))
                            .collect();
                        if !examples_vec.is_empty() {
                            examples.insert(field_name.clone(), examples_vec);
                        }
                    }
                }
            }
        }
    }

    examples
}

/// Extract default values for scope item fields from a JSON Schema
///
/// For scope-typed fields (type = "array" with items.properties), extracts
/// any default values defined for item properties.
///
/// # Arguments
///
/// * `schema` - A JSON Schema object (must have "properties" field)
///
/// # Returns
///
/// A HashMap of scope field names to their item defaults:
/// `HashMap<scope_field_name, HashMap<item_field_name, default_value>>`
pub fn extract_scope_item_defaults(
    schema: &QuillValue,
) -> HashMap<String, HashMap<String, QuillValue>> {
    let mut scope_defaults = HashMap::new();

    // Get the properties object from the schema
    if let Some(properties) = schema.as_json().get("properties") {
        if let Some(properties_obj) = properties.as_object() {
            for (field_name, field_schema) in properties_obj {
                // Check if this is a scope-typed field (array with items)
                let is_array = field_schema
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "array")
                    .unwrap_or(false);

                if !is_array {
                    continue;
                }

                // Get items schema
                if let Some(items_schema) = field_schema.get("items") {
                    // Get properties of items
                    if let Some(item_props) = items_schema.get("properties") {
                        if let Some(item_props_obj) = item_props.as_object() {
                            let mut item_defaults = HashMap::new();

                            for (item_field_name, item_field_schema) in item_props_obj {
                                // Extract default value if present
                                if let Some(default_value) = item_field_schema.get("default") {
                                    item_defaults.insert(
                                        item_field_name.clone(),
                                        QuillValue::from_json(default_value.clone()),
                                    );
                                }
                            }

                            if !item_defaults.is_empty() {
                                scope_defaults.insert(field_name.clone(), item_defaults);
                            }
                        }
                    }
                }
            }
        }
    }

    scope_defaults
}

/// Apply default values to scope item fields in a document
///
/// For each scope-typed field (arrays), iterates through items and
/// inserts default values for missing fields.
///
/// # Arguments
///
/// * `fields` - The document fields containing scope arrays
/// * `scope_defaults` - Defaults for scope items from `extract_scope_item_defaults`
///
/// # Returns
///
/// A new HashMap with default values applied to scope items
pub fn apply_scope_item_defaults(
    fields: &HashMap<String, QuillValue>,
    scope_defaults: &HashMap<String, HashMap<String, QuillValue>>,
) -> HashMap<String, QuillValue> {
    let mut result = fields.clone();

    for (scope_name, item_defaults) in scope_defaults {
        if let Some(scope_value) = result.get(scope_name) {
            // Get the array of items
            if let Some(items_array) = scope_value.as_array() {
                let mut updated_items: Vec<serde_json::Value> = Vec::new();

                for item in items_array {
                    // Get item as object
                    if let Some(item_obj) = item.as_object() {
                        let mut new_item = item_obj.clone();

                        // Apply defaults for missing fields
                        for (default_field, default_value) in item_defaults {
                            if !new_item.contains_key(default_field) {
                                new_item
                                    .insert(default_field.clone(), default_value.as_json().clone());
                            }
                        }

                        updated_items.push(serde_json::Value::Object(new_item));
                    } else {
                        // Item is not an object, keep as-is
                        updated_items.push(item.clone());
                    }
                }

                result.insert(
                    scope_name.clone(),
                    QuillValue::from_json(serde_json::Value::Array(updated_items)),
                );
            }
        }
    }

    result
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
            let path = error.instance_path().to_string();
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

/// Coerce a single value to match the expected schema type
///
/// Performs type coercions such as:
/// - Singular values to single-element arrays when schema expects array
/// - String "true"/"false" to boolean
/// - Number 0/1 to boolean
/// - String numbers to number type
/// - Boolean to number (true->1, false->0)
fn coerce_value(value: &QuillValue, expected_type: &str) -> QuillValue {
    let json_value = value.as_json();

    match expected_type {
        "array" => {
            // If value is already an array, return as-is
            if json_value.is_array() {
                return value.clone();
            }
            // Otherwise, wrap the value in a single-element array
            QuillValue::from_json(Value::Array(vec![json_value.clone()]))
        }
        "boolean" => {
            // If already a boolean, return as-is
            if let Some(b) = json_value.as_bool() {
                return QuillValue::from_json(Value::Bool(b));
            }
            // Coerce from string "true"/"false" (case-insensitive)
            if let Some(s) = json_value.as_str() {
                let lower = s.to_lowercase();
                if lower == "true" {
                    return QuillValue::from_json(Value::Bool(true));
                } else if lower == "false" {
                    return QuillValue::from_json(Value::Bool(false));
                }
            }
            // Coerce from number (0 = false, non-zero = true)
            if let Some(n) = json_value.as_i64() {
                return QuillValue::from_json(Value::Bool(n != 0));
            }
            if let Some(n) = json_value.as_f64() {
                // Handle NaN and use epsilon comparison for zero
                if n.is_nan() {
                    return QuillValue::from_json(Value::Bool(false));
                }
                return QuillValue::from_json(Value::Bool(n.abs() > f64::EPSILON));
            }
            // Can't coerce, return as-is
            value.clone()
        }
        "number" => {
            // If already a number, return as-is
            if json_value.is_number() {
                return value.clone();
            }
            // Coerce from string
            if let Some(s) = json_value.as_str() {
                // Try parsing as integer first
                if let Ok(i) = s.parse::<i64>() {
                    return QuillValue::from_json(serde_json::Number::from(i).into());
                }
                // Try parsing as float
                if let Ok(f) = s.parse::<f64>() {
                    if let Some(num) = serde_json::Number::from_f64(f) {
                        return QuillValue::from_json(num.into());
                    }
                }
            }
            // Coerce from boolean (true -> 1, false -> 0)
            if let Some(b) = json_value.as_bool() {
                let num_value = if b { 1 } else { 0 };
                return QuillValue::from_json(Value::Number(serde_json::Number::from(num_value)));
            }
            // Can't coerce, return as-is
            value.clone()
        }
        _ => {
            // For other types (string, object, etc.), no coercion needed
            value.clone()
        }
    }
}

/// Coerce document fields to match the expected schema types
///
/// This function applies type coercions to document fields based on the schema.
/// It's useful for handling flexible input formats.
///
/// # Arguments
///
/// * `schema` - A JSON Schema object (must have "properties" field)
/// * `fields` - The document fields to coerce
///
/// # Returns
///
/// A new HashMap with coerced field values
pub fn coerce_document(
    schema: &QuillValue,
    fields: &HashMap<String, QuillValue>,
) -> HashMap<String, QuillValue> {
    let mut coerced_fields = HashMap::new();

    // Get the properties object from the schema
    let properties = match schema.as_json().get("properties") {
        Some(props) => props,
        None => {
            // No properties defined, return fields as-is
            return fields.clone();
        }
    };

    let properties_obj = match properties.as_object() {
        Some(obj) => obj,
        None => {
            // Properties is not an object, return fields as-is
            return fields.clone();
        }
    };

    // Process each field
    for (field_name, field_value) in fields {
        // Check if there's a schema definition for this field
        if let Some(field_schema) = properties_obj.get(field_name) {
            // Get the expected type
            if let Some(expected_type) = field_schema.get("type").and_then(|t| t.as_str()) {
                // Apply coercion
                let coerced_value = coerce_value(field_value, expected_type);
                coerced_fields.insert(field_name.clone(), coerced_value);
                continue;
            }
        }
        // No schema or no type specified, keep the field as-is
        coerced_fields.insert(field_name.clone(), field_value.clone());
    }

    coerced_fields
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

    #[test]
    fn test_build_schema_with_example() {
        let mut fields = HashMap::new();
        let mut schema = FieldSchema::new(
            "memo_for".to_string(),
            "List of recipient organization symbols".to_string(),
        );
        schema.r#type = Some("array".to_string());
        schema.examples = Some(QuillValue::from_json(json!([[
            "ORG1/SYMBOL",
            "ORG2/SYMBOL"
        ]])));
        fields.insert("memo_for".to_string(), schema);

        let json_schema = build_schema_from_fields(&fields).unwrap().as_json().clone();

        // Verify that examples field is present in the schema
        assert!(json_schema["properties"]["memo_for"]
            .as_object()
            .unwrap()
            .contains_key("examples"));

        let example_value = &json_schema["properties"]["memo_for"]["examples"][0];
        assert_eq!(example_value, &json!(["ORG1/SYMBOL", "ORG2/SYMBOL"]));
    }

    #[test]
    fn test_build_schema_includes_default_in_properties() {
        let mut fields = HashMap::new();
        let mut schema = FieldSchema::new(
            "ice_cream".to_string(),
            "favorite ice cream flavor".to_string(),
        );
        schema.r#type = Some("string".to_string());
        schema.default = Some(QuillValue::from_json(json!("taro")));
        fields.insert("ice_cream".to_string(), schema);

        let json_schema = build_schema_from_fields(&fields).unwrap().as_json().clone();

        // Verify that default field is present in the schema
        assert!(json_schema["properties"]["ice_cream"]
            .as_object()
            .unwrap()
            .contains_key("default"));

        let default_value = &json_schema["properties"]["ice_cream"]["default"];
        assert_eq!(default_value, &json!("taro"));

        // Verify that field with default is not required
        let required_fields = json_schema["required"].as_array().unwrap();
        assert!(!required_fields.contains(&json!("ice_cream")));
    }

    #[test]
    fn test_extract_defaults_from_schema() {
        // Create a JSON schema with defaults
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Document title"
                },
                "author": {
                    "type": "string",
                    "description": "Document author",
                    "default": "Anonymous"
                },
                "status": {
                    "type": "string",
                    "description": "Document status",
                    "default": "draft"
                },
                "count": {
                    "type": "number",
                    "default": 42
                }
            },
            "required": ["title"]
        });

        let defaults = extract_defaults_from_schema(&QuillValue::from_json(schema));

        // Verify that only fields with defaults are extracted
        assert_eq!(defaults.len(), 3);
        assert!(!defaults.contains_key("title")); // no default
        assert!(defaults.contains_key("author"));
        assert!(defaults.contains_key("status"));
        assert!(defaults.contains_key("count"));

        // Verify the default values
        assert_eq!(defaults.get("author").unwrap().as_str(), Some("Anonymous"));
        assert_eq!(defaults.get("status").unwrap().as_str(), Some("draft"));
        assert_eq!(defaults.get("count").unwrap().as_json().as_i64(), Some(42));
    }

    #[test]
    fn test_extract_defaults_from_schema_empty() {
        // Schema with no defaults
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "author": {"type": "string"}
            },
            "required": ["title"]
        });

        let defaults = extract_defaults_from_schema(&QuillValue::from_json(schema));
        assert_eq!(defaults.len(), 0);
    }

    #[test]
    fn test_extract_defaults_from_schema_no_properties() {
        // Schema without properties field
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object"
        });

        let defaults = extract_defaults_from_schema(&QuillValue::from_json(schema));
        assert_eq!(defaults.len(), 0);
    }

    #[test]
    fn test_extract_examples_from_schema() {
        // Create a JSON schema with examples
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Document title"
                },
                "memo_for": {
                    "type": "array",
                    "description": "List of recipients",
                    "examples": [
                        ["ORG1/SYMBOL", "ORG2/SYMBOL"],
                        ["DEPT/OFFICE"]
                    ]
                },
                "author": {
                    "type": "string",
                    "description": "Document author",
                    "examples": ["John Doe", "Jane Smith"]
                },
                "status": {
                    "type": "string",
                    "description": "Document status"
                }
            }
        });

        let examples = extract_examples_from_schema(&QuillValue::from_json(schema));

        // Verify that only fields with examples are extracted
        assert_eq!(examples.len(), 2);
        assert!(!examples.contains_key("title")); // no examples
        assert!(examples.contains_key("memo_for"));
        assert!(examples.contains_key("author"));
        assert!(!examples.contains_key("status")); // no examples

        // Verify the example values for memo_for
        let memo_for_examples = examples.get("memo_for").unwrap();
        assert_eq!(memo_for_examples.len(), 2);
        assert_eq!(
            memo_for_examples[0].as_json(),
            &json!(["ORG1/SYMBOL", "ORG2/SYMBOL"])
        );
        assert_eq!(memo_for_examples[1].as_json(), &json!(["DEPT/OFFICE"]));

        // Verify the example values for author
        let author_examples = examples.get("author").unwrap();
        assert_eq!(author_examples.len(), 2);
        assert_eq!(author_examples[0].as_str(), Some("John Doe"));
        assert_eq!(author_examples[1].as_str(), Some("Jane Smith"));
    }

    #[test]
    fn test_extract_examples_from_schema_empty() {
        // Schema with no examples
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "author": {"type": "string"}
            }
        });

        let examples = extract_examples_from_schema(&QuillValue::from_json(schema));
        assert_eq!(examples.len(), 0);
    }

    #[test]
    fn test_extract_examples_from_schema_no_properties() {
        // Schema without properties field
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object"
        });

        let examples = extract_examples_from_schema(&QuillValue::from_json(schema));
        assert_eq!(examples.len(), 0);
    }

    #[test]
    fn test_coerce_singular_to_array() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "tags": {"type": "array"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert(
            "tags".to_string(),
            QuillValue::from_json(json!("single-tag")),
        );

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        let tags = coerced.get("tags").unwrap();
        assert!(tags.as_array().is_some());
        let tags_array = tags.as_array().unwrap();
        assert_eq!(tags_array.len(), 1);
        assert_eq!(tags_array[0].as_str().unwrap(), "single-tag");
    }

    #[test]
    fn test_coerce_array_unchanged() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "tags": {"type": "array"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert(
            "tags".to_string(),
            QuillValue::from_json(json!(["tag1", "tag2"])),
        );

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        let tags = coerced.get("tags").unwrap();
        let tags_array = tags.as_array().unwrap();
        assert_eq!(tags_array.len(), 2);
    }

    #[test]
    fn test_coerce_string_to_boolean() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "active": {"type": "boolean"},
                "enabled": {"type": "boolean"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert("active".to_string(), QuillValue::from_json(json!("true")));
        fields.insert("enabled".to_string(), QuillValue::from_json(json!("FALSE")));

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        assert_eq!(coerced.get("active").unwrap().as_bool().unwrap(), true);
        assert_eq!(coerced.get("enabled").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    fn test_coerce_number_to_boolean() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "flag1": {"type": "boolean"},
                "flag2": {"type": "boolean"},
                "flag3": {"type": "boolean"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert("flag1".to_string(), QuillValue::from_json(json!(0)));
        fields.insert("flag2".to_string(), QuillValue::from_json(json!(1)));
        fields.insert("flag3".to_string(), QuillValue::from_json(json!(42)));

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        assert_eq!(coerced.get("flag1").unwrap().as_bool().unwrap(), false);
        assert_eq!(coerced.get("flag2").unwrap().as_bool().unwrap(), true);
        assert_eq!(coerced.get("flag3").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_coerce_float_to_boolean() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "flag1": {"type": "boolean"},
                "flag2": {"type": "boolean"},
                "flag3": {"type": "boolean"},
                "flag4": {"type": "boolean"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert("flag1".to_string(), QuillValue::from_json(json!(0.0)));
        fields.insert("flag2".to_string(), QuillValue::from_json(json!(0.5)));
        fields.insert("flag3".to_string(), QuillValue::from_json(json!(-1.5)));
        // Very small number below epsilon - should be considered false
        fields.insert("flag4".to_string(), QuillValue::from_json(json!(1e-100)));

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        assert_eq!(coerced.get("flag1").unwrap().as_bool().unwrap(), false);
        assert_eq!(coerced.get("flag2").unwrap().as_bool().unwrap(), true);
        assert_eq!(coerced.get("flag3").unwrap().as_bool().unwrap(), true);
        // Very small numbers are considered false due to epsilon comparison
        assert_eq!(coerced.get("flag4").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    fn test_coerce_string_to_number() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "count": {"type": "number"},
                "price": {"type": "number"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert("count".to_string(), QuillValue::from_json(json!("42")));
        fields.insert("price".to_string(), QuillValue::from_json(json!("19.99")));

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        assert_eq!(coerced.get("count").unwrap().as_i64().unwrap(), 42);
        assert_eq!(coerced.get("price").unwrap().as_f64().unwrap(), 19.99);
    }

    #[test]
    fn test_coerce_boolean_to_number() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "active": {"type": "number"},
                "disabled": {"type": "number"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert("active".to_string(), QuillValue::from_json(json!(true)));
        fields.insert("disabled".to_string(), QuillValue::from_json(json!(false)));

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        assert_eq!(coerced.get("active").unwrap().as_i64().unwrap(), 1);
        assert_eq!(coerced.get("disabled").unwrap().as_i64().unwrap(), 0);
    }

    #[test]
    fn test_coerce_no_schema_properties() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object"
        });

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), QuillValue::from_json(json!("Test")));

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        // Fields should remain unchanged
        assert_eq!(coerced.get("title").unwrap().as_str().unwrap(), "Test");
    }

    #[test]
    fn test_coerce_field_without_type() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {"description": "A title field"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert("title".to_string(), QuillValue::from_json(json!("Test")));

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        // Field should remain unchanged when no type is specified
        assert_eq!(coerced.get("title").unwrap().as_str().unwrap(), "Test");
    }

    #[test]
    fn test_coerce_mixed_fields() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "tags": {"type": "array"},
                "active": {"type": "boolean"},
                "count": {"type": "number"},
                "title": {"type": "string"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert("tags".to_string(), QuillValue::from_json(json!("single")));
        fields.insert("active".to_string(), QuillValue::from_json(json!("true")));
        fields.insert("count".to_string(), QuillValue::from_json(json!("42")));
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(json!("Test Title")),
        );

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        // Verify coercions
        assert_eq!(coerced.get("tags").unwrap().as_array().unwrap().len(), 1);
        assert_eq!(coerced.get("active").unwrap().as_bool().unwrap(), true);
        assert_eq!(coerced.get("count").unwrap().as_i64().unwrap(), 42);
        assert_eq!(
            coerced.get("title").unwrap().as_str().unwrap(),
            "Test Title"
        );
    }

    #[test]
    fn test_coerce_invalid_string_to_number() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "count": {"type": "number"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert(
            "count".to_string(),
            QuillValue::from_json(json!("not-a-number")),
        );

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        // Should remain unchanged when coercion fails
        assert_eq!(
            coerced.get("count").unwrap().as_str().unwrap(),
            "not-a-number"
        );
    }

    #[test]
    fn test_coerce_object_to_array() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "items": {"type": "array"}
            }
        });

        let mut fields = HashMap::new();
        fields.insert(
            "items".to_string(),
            QuillValue::from_json(json!({"key": "value"})),
        );

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        // Object should be wrapped in an array
        let items = coerced.get("items").unwrap();
        assert!(items.as_array().is_some());
        let items_array = items.as_array().unwrap();
        assert_eq!(items_array.len(), 1);
        assert!(items_array[0].as_object().is_some());
    }

    #[test]
    fn test_schema_scope_generates_array() {
        // Test that type = "scope" generates JSON Schema with type = "array"
        let mut fields = HashMap::new();
        let mut scope_schema = FieldSchema::new(
            "endorsements".to_string(),
            "Chain of endorsements".to_string(),
        );
        scope_schema.r#type = Some("scope".to_string());
        scope_schema.title = Some("Endorsements".to_string());
        fields.insert("endorsements".to_string(), scope_schema);

        let json_schema = build_schema_from_fields(&fields).unwrap().as_json().clone();

        // Verify the scope field generates array type
        let endorsements = &json_schema["properties"]["endorsements"];
        assert_eq!(endorsements["type"], "array");
        assert_eq!(endorsements["name"], "endorsements");
        assert_eq!(endorsements["title"], "Endorsements");
        assert_eq!(endorsements["description"], "Chain of endorsements");

        // Verify items is an object type
        assert_eq!(endorsements["items"]["type"], "object");
    }

    #[test]
    fn test_schema_scope_items_properties() {
        // Test that scope items generate nested properties in JSON Schema
        let mut fields = HashMap::new();

        let mut scope_schema = FieldSchema::new(
            "endorsements".to_string(),
            "Chain of endorsements".to_string(),
        );
        scope_schema.r#type = Some("scope".to_string());

        // Add item schemas
        let mut name_schema = FieldSchema::new("name".to_string(), "Endorser name".to_string());
        name_schema.r#type = Some("string".to_string());
        name_schema.required = true; // Explicitly mark as required

        let mut org_schema = FieldSchema::new("org".to_string(), "Organization".to_string());
        org_schema.r#type = Some("string".to_string());
        org_schema.default = Some(QuillValue::from_json(json!("Unknown")));

        let mut items = HashMap::new();
        items.insert("name".to_string(), name_schema);
        items.insert("org".to_string(), org_schema);
        scope_schema.items = Some(items);

        fields.insert("endorsements".to_string(), scope_schema);

        let json_schema = build_schema_from_fields(&fields).unwrap().as_json().clone();
        let endorsements = &json_schema["properties"]["endorsements"];

        // Verify items has properties
        let items_schema = &endorsements["items"];
        assert_eq!(items_schema["type"], "object");
        assert!(items_schema["properties"]["name"].is_object());
        assert!(items_schema["properties"]["org"].is_object());

        // Verify item field types
        assert_eq!(items_schema["properties"]["name"]["type"], "string");
        assert_eq!(items_schema["properties"]["org"]["type"], "string");
        assert_eq!(items_schema["properties"]["org"]["default"], "Unknown");

        // Verify required propagation: name has required=true, org is optional
        let required = items_schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("name")));
        assert!(!required.contains(&json!("org")));

        // Verify additionalProperties is set
        assert_eq!(items_schema["additionalProperties"], true);
    }

    #[test]
    fn test_extract_scope_item_defaults() {
        // Create a JSON schema with scope items that have defaults
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "endorsements": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "org": { "type": "string", "default": "Unknown Org" },
                            "rank": { "type": "string", "default": "N/A" }
                        }
                    }
                },
                "title": { "type": "string" }
            }
        });

        let scope_defaults = extract_scope_item_defaults(&QuillValue::from_json(schema));

        // Should have one scope field with defaults
        assert_eq!(scope_defaults.len(), 1);
        assert!(scope_defaults.contains_key("endorsements"));

        let endorsements_defaults = scope_defaults.get("endorsements").unwrap();
        assert_eq!(endorsements_defaults.len(), 2); // org and rank have defaults
        assert!(!endorsements_defaults.contains_key("name")); // name has no default
        assert_eq!(
            endorsements_defaults.get("org").unwrap().as_str(),
            Some("Unknown Org")
        );
        assert_eq!(
            endorsements_defaults.get("rank").unwrap().as_str(),
            Some("N/A")
        );
    }

    #[test]
    fn test_extract_scope_item_defaults_empty() {
        // Schema with no scope fields
        let schema = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            }
        });

        let scope_defaults = extract_scope_item_defaults(&QuillValue::from_json(schema));
        assert!(scope_defaults.is_empty());
    }

    #[test]
    fn test_extract_scope_item_defaults_no_item_defaults() {
        // Schema with scope field but no item defaults
        let schema = json!({
            "type": "object",
            "properties": {
                "endorsements": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "org": { "type": "string" }
                        }
                    }
                }
            }
        });

        let scope_defaults = extract_scope_item_defaults(&QuillValue::from_json(schema));
        assert!(scope_defaults.is_empty()); // No defaults defined
    }

    #[test]
    fn test_apply_scope_item_defaults() {
        // Set up scope defaults
        let mut item_defaults = HashMap::new();
        item_defaults.insert(
            "org".to_string(),
            QuillValue::from_json(json!("Default Org")),
        );

        let mut scope_defaults = HashMap::new();
        scope_defaults.insert("endorsements".to_string(), item_defaults);

        // Set up document fields with scope items missing the 'org' field
        let mut fields = HashMap::new();
        fields.insert(
            "endorsements".to_string(),
            QuillValue::from_json(json!([
                { "name": "John Doe" },
                { "name": "Jane Smith", "org": "Custom Org" }
            ])),
        );

        let result = apply_scope_item_defaults(&fields, &scope_defaults);

        // Verify defaults were applied
        let endorsements = result.get("endorsements").unwrap().as_array().unwrap();
        assert_eq!(endorsements.len(), 2);

        // First item should have default applied
        assert_eq!(endorsements[0]["name"], "John Doe");
        assert_eq!(endorsements[0]["org"], "Default Org");

        // Second item should preserve existing value
        assert_eq!(endorsements[1]["name"], "Jane Smith");
        assert_eq!(endorsements[1]["org"], "Custom Org");
    }

    #[test]
    fn test_apply_scope_item_defaults_empty_scope() {
        let mut item_defaults = HashMap::new();
        item_defaults.insert(
            "org".to_string(),
            QuillValue::from_json(json!("Default Org")),
        );

        let mut scope_defaults = HashMap::new();
        scope_defaults.insert("endorsements".to_string(), item_defaults);

        // Empty endorsements array
        let mut fields = HashMap::new();
        fields.insert("endorsements".to_string(), QuillValue::from_json(json!([])));

        let result = apply_scope_item_defaults(&fields, &scope_defaults);

        // Should still be empty array
        let endorsements = result.get("endorsements").unwrap().as_array().unwrap();
        assert!(endorsements.is_empty());
    }

    #[test]
    fn test_apply_scope_item_defaults_no_matching_scope() {
        let mut item_defaults = HashMap::new();
        item_defaults.insert(
            "org".to_string(),
            QuillValue::from_json(json!("Default Org")),
        );

        let mut scope_defaults = HashMap::new();
        scope_defaults.insert("endorsements".to_string(), item_defaults);

        // Document has different scope field
        let mut fields = HashMap::new();
        fields.insert(
            "reviews".to_string(),
            QuillValue::from_json(json!([{ "author": "Bob" }])),
        );

        let result = apply_scope_item_defaults(&fields, &scope_defaults);

        // reviews should be unchanged
        let reviews = result.get("reviews").unwrap().as_array().unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0]["author"], "Bob");
        assert!(reviews[0].get("org").is_none());
    }

    #[test]
    fn test_scope_validation_with_required_fields() {
        // Test that JSON Schema validation rejects scope items missing required fields
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "endorsements": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "org": { "type": "string", "default": "Unknown" }
                        },
                        "required": ["name"]
                    }
                }
            }
        });

        // Valid: has required 'name' field
        let mut valid_fields = HashMap::new();
        valid_fields.insert(
            "endorsements".to_string(),
            QuillValue::from_json(json!([{ "name": "John" }])),
        );

        let result = validate_document(&QuillValue::from_json(schema.clone()), &valid_fields);
        assert!(result.is_ok());

        // Invalid: missing required 'name' field
        let mut invalid_fields = HashMap::new();
        invalid_fields.insert(
            "endorsements".to_string(),
            QuillValue::from_json(json!([{ "org": "SomeOrg" }])),
        );

        let result = validate_document(&QuillValue::from_json(schema), &invalid_fields);
        assert!(result.is_err());
    }
}
