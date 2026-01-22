use quillmark_core::{
    normalize::normalize_document,
    quill::{CardSchema, FieldSchema, FieldType},
    schema::build_schema,
    ParsedDocument, QuillValue,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_markdown_field_schema_generation() {
    let mut fields = HashMap::new();
    fields.insert(
        "description".to_string(),
        FieldSchema {
            name: "description".to_string(),
            title: Some("Description".to_string()),
            r#type: FieldType::Markdown,
            description: None,
            default: None,
            examples: None,
            ui: None,
            required: false,
            enum_values: None,
            properties: None,
            items: None,
        },
    );

    let doc_schema = CardSchema {
        name: "root".to_string(),
        title: None,
        description: None,
        fields,
        ui: None,
    };

    let schema = build_schema(&doc_schema, &HashMap::new()).unwrap();
    let schema_json = schema.as_json();

    let desc_prop = schema_json
        .get("properties")
        .expect("should have properties")
        .get("description")
        .expect("should have description property");

    assert_eq!(
        desc_prop.get("type").and_then(|v| v.as_str()),
        Some("string"),
        "Markdown field should be type string in JSON Schema"
    );

    assert_eq!(
        desc_prop.get("contentMediaType").and_then(|v| v.as_str()),
        Some("text/markdown"),
        "Markdown field should have contentMediaType set to text/markdown"
    );
}

#[test]
fn test_markdown_field_normalization() {
    // 1. Define schema with a Markdown field and a String field
    let mut fields = HashMap::new();
    fields.insert(
        "markdown_field".to_string(),
        FieldSchema {
            name: "markdown_field".to_string(),
            title: None,
            r#type: FieldType::Markdown,
            description: None,
            default: None,
            examples: None,
            ui: None,
            required: false,
            enum_values: None,
            properties: None,
            items: None,
        },
    );
    fields.insert(
        "string_field".to_string(),
        FieldSchema {
            name: "string_field".to_string(),
            title: None,
            r#type: FieldType::String,
            description: None,
            default: None,
            examples: None,
            ui: None,
            required: false,
            enum_values: None,
            properties: None,
            items: None,
        },
    );

    let doc_schema = CardSchema {
        name: "root".to_string(),
        title: None,
        description: None,
        fields,
        ui: None,
    };

    let schema = build_schema(&doc_schema, &HashMap::new()).unwrap();

    // 2. Create a document with chevrons in both fields
    let mut doc_fields = HashMap::new();
    doc_fields.insert(
        "markdown_field".to_string(),
        QuillValue::from_json(json!("This has <<guillemets>>")),
    );
    doc_fields.insert(
        "string_field".to_string(),
        QuillValue::from_json(json!("This has <<stripped>>")),
    );

    let doc = ParsedDocument::new(doc_fields);

    // 3. Normalize (schema no longer affects normalization)
    let _ = schema; // Schema is built for the first test but not needed here
    let normalized = normalize_document(doc).expect("Failed to normalize document");
    let norm_fields = normalized.fields();

    // 4. Verify results
    // Markdown field: chevrons pass through unchanged
    assert_eq!(
        norm_fields.get("markdown_field").unwrap().as_str().unwrap(),
        "This has <<guillemets>>"
    );

    // String field: chevrons also pass through unchanged
    assert_eq!(
        norm_fields.get("string_field").unwrap().as_str().unwrap(),
        "This has <<stripped>>"
    );
}
