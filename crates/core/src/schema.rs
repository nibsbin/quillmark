//! Schema validation and utilities for Quillmark.
//!
//! This module provides utilities for converting TOML field definitions to JSON Schema
//! and validating ParsedDocument data against schemas.

use crate::quill::{field_key, ui_key, CardSchema, FieldSchema, FieldType};
use crate::{QuillValue, RenderError};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// Build a single field property JSON Schema object from a FieldSchema
fn build_field_property(field_schema: &FieldSchema) -> Map<String, Value> {
    let mut property = Map::new();

    // Add name
    property.insert(
        field_key::NAME.to_string(),
        Value::String(field_schema.name.clone()),
    );

    // Map field type to JSON Schema type
    if let Some(ref field_type) = field_schema.r#type {
        let (json_type, format) = match field_type {
            FieldType::Str | FieldType::String => ("string", None),
            FieldType::Number => ("number", None),
            FieldType::Boolean => ("boolean", None),
            FieldType::Array => ("array", None),
            FieldType::Dict => ("object", None),
            FieldType::Date => ("string", Some("date")),
            FieldType::DateTime => ("string", Some("date-time")),
        };
        property.insert(
            field_key::TYPE.to_string(),
            Value::String(json_type.to_string()),
        );

        // Add format for date types
        if let Some(fmt) = format {
            property.insert(
                field_key::FORMAT.to_string(),
                Value::String(fmt.to_string()),
            );
        }
    }

    // Add title if specified
    if let Some(ref title) = field_schema.title {
        property.insert(field_key::TITLE.to_string(), Value::String(title.clone()));
    }

    // Add description
    property.insert(
        field_key::DESCRIPTION.to_string(),
        Value::String(field_schema.description.clone()),
    );

    // Add UI metadata as x-ui property if present
    if let Some(ref ui) = field_schema.ui {
        let mut ui_obj = Map::new();

        if let Some(ref group) = ui.group {
            ui_obj.insert(ui_key::GROUP.to_string(), Value::String(group.clone()));
        }

        if let Some(order) = ui.order {
            ui_obj.insert(ui_key::ORDER.to_string(), json!(order));
        }

        if !ui_obj.is_empty() {
            property.insert("x-ui".to_string(), Value::Object(ui_obj));
        }
    }

    // Add examples if specified
    if let Some(ref examples) = field_schema.examples {
        if let Some(examples_array) = examples.as_array() {
            if !examples_array.is_empty() {
                property.insert(
                    field_key::EXAMPLES.to_string(),
                    Value::Array(examples_array.clone()),
                );
            }
        }
    }

    // Add default if specified
    if let Some(ref default) = field_schema.default {
        property.insert(field_key::DEFAULT.to_string(), default.as_json().clone());
    }

    // Add enum constraint if specified (for string types)
    if let Some(ref enum_values) = field_schema.enum_values {
        let enum_array: Vec<Value> = enum_values
            .iter()
            .map(|s| Value::String(s.clone()))
            .collect();
        property.insert(field_key::ENUM.to_string(), Value::Array(enum_array));
    }

    property
}

/// Build a card schema definition for `$defs`
fn build_card_def(name: &str, card: &CardSchema) -> Map<String, Value> {
    let mut def = Map::new();

    def.insert("type".to_string(), Value::String("object".to_string()));

    // Add title if specified
    if let Some(ref title) = card.title {
        def.insert("title".to_string(), Value::String(title.clone()));
    }

    // Add description
    if !card.description.is_empty() {
        def.insert(
            "description".to_string(),
            Value::String(card.description.clone()),
        );
    }

    // Build properties
    let mut properties = Map::new();
    let mut required = vec![Value::String("CARD".to_string())];

    // Add CARD discriminator property
    let mut card_prop = Map::new();
    card_prop.insert("const".to_string(), Value::String(name.to_string()));
    properties.insert("CARD".to_string(), Value::Object(card_prop));

    // Add card field properties
    for (field_name, field_schema) in &card.fields {
        let field_prop = build_field_property(field_schema);
        properties.insert(field_name.clone(), Value::Object(field_prop));

        if field_schema.required {
            required.push(Value::String(field_name.clone()));
        }
    }

    def.insert("properties".to_string(), Value::Object(properties));
    def.insert("required".to_string(), Value::Array(required));
    def.insert("additionalProperties".to_string(), Value::Bool(true));

    def
}

/// Build a JSON Schema from field and card schemas
///
/// Generates a JSON Schema with:
/// - Regular fields in `properties`
/// - Card schemas in `$defs`
/// - `CARDS` array with `oneOf` refs and `x-discriminator`
pub fn build_schema(
    field_schemas: &HashMap<String, FieldSchema>,
    card_schemas: &HashMap<String, CardSchema>,
) -> Result<QuillValue, RenderError> {
    let mut properties = Map::new();
    let mut required_fields = Vec::new();
    let mut defs = Map::new();

    // Build field properties
    for (field_name, field_schema) in field_schemas {
        let property = build_field_property(field_schema);
        properties.insert(field_name.clone(), Value::Object(property));

        if field_schema.required {
            required_fields.push(field_name.clone());
        }
    }

    // Build card definitions and CARDS array
    if !card_schemas.is_empty() {
        let mut one_of = Vec::new();
        let mut discriminator_mapping = Map::new();

        for (card_name, card_schema) in card_schemas {
            let def_name = format!("{}_card", card_name);
            let ref_path = format!("#/$defs/{}", def_name);

            // Add to $defs
            defs.insert(
                def_name.clone(),
                Value::Object(build_card_def(card_name, card_schema)),
            );

            // Add to oneOf
            let mut ref_obj = Map::new();
            ref_obj.insert("$ref".to_string(), Value::String(ref_path.clone()));
            one_of.push(Value::Object(ref_obj));

            // Add to discriminator mapping
            discriminator_mapping.insert(card_name.clone(), Value::String(ref_path));
        }

        // Build CARDS array property
        let mut items_schema = Map::new();
        items_schema.insert("oneOf".to_string(), Value::Array(one_of));

        // x-discriminator removed in favor of const polymorphism

        let mut cards_property = Map::new();
        cards_property.insert("type".to_string(), Value::String("array".to_string()));
        cards_property.insert("items".to_string(), Value::Object(items_schema));

        properties.insert("CARDS".to_string(), Value::Object(cards_property));
    }

    // Build the complete JSON Schema
    let mut schema_map = Map::new();
    schema_map.insert(
        "$schema".to_string(),
        Value::String("https://json-schema.org/draft/2019-09/schema".to_string()),
    );
    schema_map.insert("type".to_string(), Value::String("object".to_string()));

    // Add $defs if there are card schemas
    if !defs.is_empty() {
        schema_map.insert("$defs".to_string(), Value::Object(defs));
    }

    schema_map.insert("properties".to_string(), Value::Object(properties));
    schema_map.insert(
        "required".to_string(),
        Value::Array(required_fields.into_iter().map(Value::String).collect()),
    );
    schema_map.insert("additionalProperties".to_string(), Value::Bool(true));

    let schema = Value::Object(schema_map);

    Ok(QuillValue::from_json(schema))
}

/// Backwards-compatible wrapper for build_schema (no cards)
pub fn build_schema_from_fields(
    field_schemas: &HashMap<String, FieldSchema>,
) -> Result<QuillValue, RenderError> {
    build_schema(field_schemas, &HashMap::new())
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

/// Extract default values for card item fields from a JSON Schema
///
/// For card-typed fields (type = "array" with items.properties), extracts
/// any default values defined for item properties.
///
/// # Arguments
///
/// * `schema` - A JSON Schema object (must have "properties" field)
///
/// # Returns
///
/// A HashMap of card field names to their item defaults:
/// `HashMap<card_field_name, HashMap<item_field_name, default_value>>`
pub fn extract_card_item_defaults(
    schema: &QuillValue,
) -> HashMap<String, HashMap<String, QuillValue>> {
    let mut card_defaults = HashMap::new();

    // Get the properties object from the schema
    if let Some(properties) = schema.as_json().get("properties") {
        if let Some(properties_obj) = properties.as_object() {
            for (field_name, field_schema) in properties_obj {
                // Check if this is a card-typed field (array with items)
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
                                card_defaults.insert(field_name.clone(), item_defaults);
                            }
                        }
                    }
                }
            }
        }
    }

    card_defaults
}

/// Apply default values to card item fields in a document
///
/// For each card-typed field (arrays), iterates through items and
/// inserts default values for missing fields.
///
/// # Arguments
///
/// * `fields` - The document fields containing card arrays
/// * `card_defaults` - Defaults for card items from `extract_card_item_defaults`
///
/// # Returns
///
/// A new HashMap with default values applied to card items
pub fn apply_card_item_defaults(
    fields: &HashMap<String, QuillValue>,
    card_defaults: &HashMap<String, HashMap<String, QuillValue>>,
) -> HashMap<String, QuillValue> {
    let mut result = fields.clone();

    for (card_name, item_defaults) in card_defaults {
        if let Some(card_value) = result.get(card_name) {
            // Get the array of items
            if let Some(items_array) = card_value.as_array() {
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
                    card_name.clone(),
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
    let mut all_errors = Vec::new();

    // 1. Recursive card validation
    if let Some(cards) = doc_value.get("CARDS").and_then(|v| v.as_array()) {
        let card_errors = validate_cards_array(schema, cards);
        all_errors.extend(card_errors);
    }

    // 2. Standard validation
    let validation_result = compiled.validate(&doc_value);

    match validation_result {
        Ok(_) => {
            if all_errors.is_empty() {
                Ok(())
            } else {
                Err(all_errors)
            }
        }
        Err(error) => {
            let path = error.instance_path().to_string();
            let path_display = if path.is_empty() {
                "document".to_string()
            } else {
                path.clone()
            };

            // If we have specific card errors, we might want to skip generic CARDS errors
            // from the main schema validation to avoid noise.
            // But for now, we'll include everything unless it's a "oneOf" error on a card we already diagnosed.
            let is_generic_card_error = path.starts_with("/CARDS/")
                && error.to_string().contains("oneOf")
                && !all_errors.is_empty();

            if !is_generic_card_error {
                // Check for potential invalid card type error (legacy check, but still useful)
                if path.starts_with("/CARDS/") && error.to_string().contains("oneOf") {
                    // Try to parse the index from path /CARDS/n
                    if let Some(rest) = path.strip_prefix("/CARDS/") {
                        // path might be just "/CARDS/0" or "/CARDS/0/some/field"
                        // We only want to intervene if the error is about the card item itself failing oneOf
                        let is_item_error = !rest.contains('/');

                        if is_item_error {
                            if let Ok(idx) = rest.parse::<usize>() {
                                if let Some(cards) =
                                    doc_value.get("CARDS").and_then(|v| v.as_array())
                                {
                                    if let Some(item) = cards.get(idx) {
                                        // Check if the item has a CARD field
                                        if let Some(card_type) =
                                            item.get("CARD").and_then(|v| v.as_str())
                                        {
                                            // Collect valid card types from schema definitions
                                            let mut valid_types = Vec::new();
                                            if let Some(defs) = schema
                                                .as_json()
                                                .get("$defs")
                                                .and_then(|v| v.as_object())
                                            {
                                                for key in defs.keys() {
                                                    if let Some(name) = key.strip_suffix("_card") {
                                                        valid_types.push(name.to_string());
                                                    }
                                                }
                                            }

                                            // If we found valid types and the current type is NOT in the list
                                            if !valid_types.is_empty()
                                                && !valid_types.contains(&card_type.to_string())
                                            {
                                                valid_types.sort();
                                                let valid_list = valid_types.join(", ");
                                                let message = format!("Validation error at {}: Invalid card type '{}'. Valid types are: [{}]", path_display, card_type, valid_list);
                                                all_errors.push(message);
                                                return Err(all_errors);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let message = format!("Validation error at {}: {}", path_display, error);
                all_errors.push(message);
            }

            Err(all_errors)
        }
    }
}

/// Helper to recursively validate an array of card objects
fn validate_cards_array(document_schema: &QuillValue, cards_array: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();

    // Get definitions for card schemas
    let defs = document_schema
        .as_json()
        .get("$defs")
        .and_then(|v| v.as_object());

    for (idx, card) in cards_array.iter().enumerate() {
        // We only process objects that have a CARD discriminator
        if let Some(card_obj) = card.as_object() {
            if let Some(card_type) = card_obj.get("CARD").and_then(|v| v.as_str()) {
                // Construct the definition name: {type}_card
                let def_name = format!("{}_card", card_type);

                // Look up the schema for this card type
                if let Some(card_schema_json) = defs.and_then(|d| d.get(&def_name)) {
                    // Convert the card object to HashMap<String, QuillValue> for recursion
                    let mut card_fields = HashMap::new();
                    for (k, v) in card_obj {
                        card_fields.insert(k.clone(), QuillValue::from_json(v.clone()));
                    }

                    // Recursively validate this card's fields
                    if let Err(card_errors) = validate_document(
                        &QuillValue::from_json(card_schema_json.clone()),
                        &card_fields,
                    ) {
                        // Prefix errors with location
                        for err in card_errors {
                            // If the error already starts with "Validation error at ", insert the prefix
                            // otherwise just prefix it.
                            // Typical error: "Validation error at field: message"
                            // We want: "Validation error at /CARDS/0/field: message"

                            let prefix = format!("/CARDS/{}", idx);
                            let new_msg =
                                if let Some(rest) = err.strip_prefix("Validation error at ") {
                                    if rest.starts_with("document") {
                                        // "Validation error at document: message" -> "Validation error at /CARDS/0: message"
                                        format!(
                                            "Validation error at {}:{}",
                                            prefix,
                                            rest.strip_prefix("document").unwrap_or(rest)
                                        )
                                    } else {
                                        // "Validation error at /field: message" -> "Validation error at /CARDS/0/field: message"
                                        format!("Validation error at {}{}", prefix, rest)
                                    }
                                } else {
                                    format!("Validation error at {}: {}", prefix, err)
                                };

                            errors.push(new_msg);
                        }
                    }
                }
            }
        }
    }

    errors
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
        "string" => {
            // If already a string, return as-is
            if json_value.is_string() {
                return value.clone();
            }
            // Coerce from single-item array (unwrap)
            if let Some(arr) = json_value.as_array() {
                if arr.len() == 1 {
                    if let Some(s) = arr[0].as_str() {
                        return QuillValue::from_json(Value::String(s.to_string()));
                    }
                }
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

    // Recursively coerce cards if the CARDS field is present
    if let Some(cards_value) = coerced_fields.get("CARDS") {
        if let Some(cards_array) = cards_value.as_array() {
            let coerced_cards = coerce_cards_array(schema, cards_array);
            coerced_fields.insert(
                "CARDS".to_string(),
                QuillValue::from_json(Value::Array(coerced_cards)),
            );
        }
    }

    coerced_fields
}

/// Helper to recursively coerce an array of card objects
fn coerce_cards_array(document_schema: &QuillValue, cards_array: &[Value]) -> Vec<Value> {
    let mut coerced_cards = Vec::new();

    // Get definitions for card schemas
    let defs = document_schema
        .as_json()
        .get("$defs")
        .and_then(|v| v.as_object());

    for card in cards_array {
        // We only process objects that have a CARD discriminator
        if let Some(card_obj) = card.as_object() {
            if let Some(card_type) = card_obj.get("CARD").and_then(|v| v.as_str()) {
                // Construct the definition name: {type}_card
                let def_name = format!("{}_card", card_type);

                // Look up the schema for this card type
                if let Some(card_schema_json) = defs.and_then(|d| d.get(&def_name)) {
                    // Convert the card object to HashMap<String, QuillValue> for coerce_document
                    let mut card_fields = HashMap::new();
                    for (k, v) in card_obj {
                        card_fields.insert(k.clone(), QuillValue::from_json(v.clone()));
                    }

                    // Recursively coerce this card's fields
                    let coerced_card_fields = coerce_document(
                        &QuillValue::from_json(card_schema_json.clone()),
                        &card_fields,
                    );

                    // Convert back to JSON Value
                    let mut coerced_card_obj = Map::new();
                    for (k, v) in coerced_card_fields {
                        coerced_card_obj.insert(k, v.into_json());
                    }

                    coerced_cards.push(Value::Object(coerced_card_obj));
                    continue;
                }
            }
        }

        // If not an object, no CARD type, or no matching schema, keep as-is
        coerced_cards.push(card.clone());
    }

    coerced_cards
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
        schema.r#type = Some(FieldType::Str);
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
        schema.r#type = Some(FieldType::Str);
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
        date_schema.r#type = Some(FieldType::Date);
        fields.insert("date_field".to_string(), date_schema);

        let mut datetime_schema = FieldSchema::new(
            "DateTime field".to_string(),
            "A field for date and time".to_string(),
        );
        datetime_schema.r#type = Some(FieldType::DateTime);
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
        schema.r#type = Some(FieldType::Array);
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
        schema.r#type = Some(FieldType::String);
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

        assert!(coerced.get("active").unwrap().as_bool().unwrap());
        assert!(!coerced.get("enabled").unwrap().as_bool().unwrap());
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

        assert!(!coerced.get("flag1").unwrap().as_bool().unwrap());
        assert!(coerced.get("flag2").unwrap().as_bool().unwrap());
        assert!(coerced.get("flag3").unwrap().as_bool().unwrap());
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

        assert!(!coerced.get("flag1").unwrap().as_bool().unwrap());
        assert!(coerced.get("flag2").unwrap().as_bool().unwrap());
        assert!(coerced.get("flag3").unwrap().as_bool().unwrap());
        // Very small numbers are considered false due to epsilon comparison
        assert!(!coerced.get("flag4").unwrap().as_bool().unwrap());
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
    fn test_coerce_array_to_string() {
        let schema = json!({
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "tags": {"type": "string"} // Incorrectly typed as string, but input is array
            }
        });

        let mut fields = HashMap::new();
        // Case 1: Single item string array -> should unwrap
        fields.insert(
            "title".to_string(),
            QuillValue::from_json(json!(["Wrapped Title"])),
        );
        // Case 2: Multi-item array -> should NOT unwrap
        fields.insert(
            "tags".to_string(),
            QuillValue::from_json(json!(["tag1", "tag2"])),
        );

        let coerced = coerce_document(&QuillValue::from_json(schema), &fields);

        // Verify unwrapping
        assert_eq!(
            coerced.get("title").unwrap().as_str().unwrap(),
            "Wrapped Title"
        );

        // Verify others left alone
        assert!(coerced.get("tags").unwrap().as_array().is_some());
        assert_eq!(coerced.get("tags").unwrap().as_array().unwrap().len(), 2);
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
        assert!(coerced.get("active").unwrap().as_bool().unwrap());
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
    fn test_schema_card_in_defs() {
        // Test that cards are generated in $defs with discriminator
        use crate::quill::CardSchema;

        let fields = HashMap::new();
        let mut cards = HashMap::new();

        let mut name_schema = FieldSchema::new("name".to_string(), "Name field".to_string());
        name_schema.r#type = Some(FieldType::String);

        let mut card_fields = HashMap::new();
        card_fields.insert("name".to_string(), name_schema);

        let card = CardSchema {
            name: "endorsements".to_string(),
            title: Some("Endorsements".to_string()),
            description: "Chain of endorsements".to_string(),
            fields: card_fields,
        };
        cards.insert("endorsements".to_string(), card);

        let json_schema = build_schema(&fields, &cards).unwrap().as_json().clone();

        // Verify $defs exists
        assert!(json_schema["$defs"].is_object());
        assert!(json_schema["$defs"]["endorsements_card"].is_object());

        // Verify card in $defs has correct structure
        let card_def = &json_schema["$defs"]["endorsements_card"];
        assert_eq!(card_def["type"], "object");
        assert_eq!(card_def["title"], "Endorsements");
        assert_eq!(card_def["description"], "Chain of endorsements");

        // Verify CARD discriminator const
        assert_eq!(card_def["properties"]["CARD"]["const"], "endorsements");

        // Verify field is in properties
        assert!(card_def["properties"]["name"].is_object());
        assert_eq!(card_def["properties"]["name"]["type"], "string");

        // Verify CARD is required
        let required = card_def["required"].as_array().unwrap();
        assert!(required.contains(&json!("CARD")));
    }

    #[test]
    fn test_schema_cards_array() {
        // Test that CARDS array is generated with oneOf but without x-discriminator
        use crate::quill::CardSchema;

        let fields = HashMap::new();
        let mut cards = HashMap::new();

        let mut name_schema = FieldSchema::new("name".to_string(), "Endorser name".to_string());
        name_schema.r#type = Some(FieldType::String);
        name_schema.required = true;

        let mut org_schema = FieldSchema::new("org".to_string(), "Organization".to_string());
        org_schema.r#type = Some(FieldType::String);
        org_schema.default = Some(QuillValue::from_json(json!("Unknown")));

        let mut card_fields = HashMap::new();
        card_fields.insert("name".to_string(), name_schema);
        card_fields.insert("org".to_string(), org_schema);

        let card = CardSchema {
            name: "endorsements".to_string(),
            title: Some("Endorsements".to_string()),
            description: "Chain of endorsements".to_string(),
            fields: card_fields,
        };
        cards.insert("endorsements".to_string(), card);

        let json_schema = build_schema(&fields, &cards).unwrap().as_json().clone();

        // Verify CARDS array property exists
        let cards_prop = &json_schema["properties"]["CARDS"];
        assert_eq!(cards_prop["type"], "array");

        // Verify items has oneOf with $ref
        let items = &cards_prop["items"];
        assert!(items["oneOf"].is_array());
        let one_of = items["oneOf"].as_array().unwrap();
        assert!(!one_of.is_empty());
        assert_eq!(one_of[0]["$ref"], "#/$defs/endorsements_card");

        // Verify x-discriminator is NOT present
        assert!(items.get("x-discriminator").is_none());

        // Verify card field properties in $defs
        let card_def = &json_schema["$defs"]["endorsements_card"];
        assert_eq!(card_def["properties"]["name"]["type"], "string");
        assert_eq!(card_def["properties"]["org"]["default"], "Unknown");

        // Verify required includes name (marked as required) and CARD
        let required = card_def["required"].as_array().unwrap();
        assert!(required.contains(&json!("CARD")));
        assert!(required.contains(&json!("name")));
        assert!(!required.contains(&json!("org")));

        // Verify additionalProperties is set
        assert_eq!(card_def["additionalProperties"], true);
    }

    #[test]
    fn test_extract_card_item_defaults() {
        // Create a JSON schema with card items that have defaults
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

        let card_defaults = extract_card_item_defaults(&QuillValue::from_json(schema));

        // Should have one card field with defaults
        assert_eq!(card_defaults.len(), 1);
        assert!(card_defaults.contains_key("endorsements"));

        let endorsements_defaults = card_defaults.get("endorsements").unwrap();
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
    fn test_extract_card_item_defaults_empty() {
        // Schema with no card fields
        let schema = json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" }
            }
        });

        let card_defaults = extract_card_item_defaults(&QuillValue::from_json(schema));
        assert!(card_defaults.is_empty());
    }

    #[test]
    fn test_extract_card_item_defaults_no_item_defaults() {
        // Schema with card field but no item defaults
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

        let card_defaults = extract_card_item_defaults(&QuillValue::from_json(schema));
        assert!(card_defaults.is_empty()); // No defaults defined
    }

    #[test]
    fn test_apply_card_item_defaults() {
        // Set up card defaults
        let mut item_defaults = HashMap::new();
        item_defaults.insert(
            "org".to_string(),
            QuillValue::from_json(json!("Default Org")),
        );

        let mut card_defaults = HashMap::new();
        card_defaults.insert("endorsements".to_string(), item_defaults);

        // Set up document fields with card items missing the 'org' field
        let mut fields = HashMap::new();
        fields.insert(
            "endorsements".to_string(),
            QuillValue::from_json(json!([
                { "name": "John Doe" },
                { "name": "Jane Smith", "org": "Custom Org" }
            ])),
        );

        let result = apply_card_item_defaults(&fields, &card_defaults);

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
    fn test_apply_card_item_defaults_empty_card() {
        let mut item_defaults = HashMap::new();
        item_defaults.insert(
            "org".to_string(),
            QuillValue::from_json(json!("Default Org")),
        );

        let mut card_defaults = HashMap::new();
        card_defaults.insert("endorsements".to_string(), item_defaults);

        // Empty endorsements array
        let mut fields = HashMap::new();
        fields.insert("endorsements".to_string(), QuillValue::from_json(json!([])));

        let result = apply_card_item_defaults(&fields, &card_defaults);

        // Should still be empty array
        let endorsements = result.get("endorsements").unwrap().as_array().unwrap();
        assert!(endorsements.is_empty());
    }

    #[test]
    fn test_apply_card_item_defaults_no_matching_card() {
        let mut item_defaults = HashMap::new();
        item_defaults.insert(
            "org".to_string(),
            QuillValue::from_json(json!("Default Org")),
        );

        let mut card_defaults = HashMap::new();
        card_defaults.insert("endorsements".to_string(), item_defaults);

        // Document has different card field
        let mut fields = HashMap::new();
        fields.insert(
            "reviews".to_string(),
            QuillValue::from_json(json!([{ "author": "Bob" }])),
        );

        let result = apply_card_item_defaults(&fields, &card_defaults);

        // reviews should be unchanged
        let reviews = result.get("reviews").unwrap().as_array().unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0]["author"], "Bob");
        assert!(reviews[0].get("org").is_none());
    }

    #[test]
    fn test_card_validation_with_required_fields() {
        // Test that JSON Schema validation rejects card items missing required fields
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
    #[test]
    fn test_validate_document_invalid_card_type() {
        use crate::quill::{CardSchema, FieldSchema};

        let mut card_fields = HashMap::new();
        card_fields.insert(
            "field1".to_string(),
            FieldSchema::new("f1".to_string(), "desc".to_string()),
        );
        let mut card_schemas = HashMap::new();
        card_schemas.insert(
            "valid_card".to_string(),
            CardSchema {
                name: "valid_card".to_string(),
                title: None,
                description: "".to_string(),
                fields: card_fields,
            },
        );

        let schema = build_schema(&HashMap::new(), &card_schemas).unwrap();

        let mut fields = HashMap::new();
        // invalid card
        let invalid_card = json!({
            "CARD": "invalid_type",
            "field1": "value" // field1 is valid for valid_card but type is wrong
        });
        fields.insert(
            "CARDS".to_string(),
            QuillValue::from_json(json!([invalid_card])),
        );

        let result = validate_document(&QuillValue::from_json(schema.as_json().clone()), &fields);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        // Check for specific improved message
        let err_msg = &errs[0];
        assert!(err_msg.contains("Invalid card type 'invalid_type'"));
        assert!(err_msg.contains("Valid types are: [valid_card]"));
    }

    #[test]
    fn test_coerce_document_cards() {
        let mut card_fields = HashMap::new();
        let mut count_schema = FieldSchema::new("Count".to_string(), "A number".to_string());
        count_schema.r#type = Some(FieldType::Number);
        card_fields.insert("count".to_string(), count_schema);

        let mut active_schema = FieldSchema::new("Active".to_string(), "A boolean".to_string());
        active_schema.r#type = Some(FieldType::Boolean);
        card_fields.insert("active".to_string(), active_schema);

        let mut card_schemas = HashMap::new();
        card_schemas.insert(
            "test_card".to_string(),
            CardSchema {
                name: "test_card".to_string(),
                title: None,
                description: "Test card".to_string(),
                fields: card_fields,
            },
        );

        let schema = build_schema(&HashMap::new(), &card_schemas).unwrap();

        let mut fields = HashMap::new();
        let card_value = json!({
            "CARD": "test_card",
            "count": "42",
            "active": "true"
        });
        fields.insert(
            "CARDS".to_string(),
            QuillValue::from_json(json!([card_value])),
        );

        let coerced_fields = coerce_document(&schema, &fields);

        let cards_array = coerced_fields.get("CARDS").unwrap().as_array().unwrap();
        let coerced_card = cards_array[0].as_object().unwrap();

        assert_eq!(coerced_card.get("count").unwrap().as_i64(), Some(42));
        assert_eq!(coerced_card.get("active").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_validate_document_card_fields() {
        let mut card_fields = HashMap::new();
        let mut count_schema = FieldSchema::new("Count".to_string(), "A number".to_string());
        count_schema.r#type = Some(FieldType::Number);
        card_fields.insert("count".to_string(), count_schema);

        let mut card_schemas = HashMap::new();
        card_schemas.insert(
            "test_card".to_string(),
            CardSchema {
                name: "test_card".to_string(),
                title: None,
                description: "Test card".to_string(),
                fields: card_fields,
            },
        );

        let schema = build_schema(&HashMap::new(), &card_schemas).unwrap();

        let mut fields = HashMap::new();
        let card_value = json!({
            "CARD": "test_card",
            "count": "not a number" // Invalid type
        });
        fields.insert(
            "CARDS".to_string(),
            QuillValue::from_json(json!([card_value])),
        );

        let result = validate_document(&QuillValue::from_json(schema.as_json().clone()), &fields);
        assert!(result.is_err());
        let errs = result.unwrap_err();

        // We expect a specific error from recursive validation
        let found_specific_error = errs
            .iter()
            .any(|e| e.contains("/CARDS/0") && e.contains("not a number") && !e.contains("oneOf"));

        assert!(
            found_specific_error,
            "Did not find specific error msg in: {:?}",
            errs
        );
    }

    #[test]
    fn test_card_field_ui_metadata() {
        // Verify that card fields with ui.group produce x-ui in JSON schema
        use crate::quill::{CardSchema, UiSchema};

        let mut field_schema = FieldSchema::new("from".to_string(), "Sender".to_string());
        field_schema.r#type = Some(FieldType::String);
        field_schema.ui = Some(UiSchema {
            group: Some("Header".to_string()),
            order: Some(0),
        });

        let mut card_fields = HashMap::new();
        card_fields.insert("from".to_string(), field_schema);

        let card = CardSchema {
            name: "indorsement".to_string(),
            title: Some("Indorsement".to_string()),
            description: "An indorsement".to_string(),
            fields: card_fields,
        };

        let mut cards = HashMap::new();
        cards.insert("indorsement".to_string(), card);

        let schema = build_schema(&HashMap::new(), &cards).unwrap();
        let card_def = &schema.as_json()["$defs"]["indorsement_card"];
        let from_field = &card_def["properties"]["from"];

        assert_eq!(from_field["x-ui"]["group"], "Header");
        assert_eq!(from_field["x-ui"]["order"], 0);
    }
}
