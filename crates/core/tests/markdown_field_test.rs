use quillmark_core::{
    normalize::normalize_document, quill::QuillConfig, version::QuillReference, ParsedDocument,
    QuillValue,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_markdown_field_public_schema_emission() {
    let config = QuillConfig::from_yaml(
        r#"
Quill:
  name: markdown_schema
  version: "1.0"
  backend: typst
  description: markdown schema test

main:
  fields:
    description:
      type: markdown
"#,
    )
    .unwrap();

    let yaml = config.public_schema_yaml().unwrap();
    let value: serde_json::Value = serde_saphyr::from_str(&yaml).unwrap();

    assert_eq!(
        value
            .get("fields")
            .and_then(|v| v.get("description"))
            .and_then(|v| v.get("type"))
            .and_then(|v| v.as_str()),
        Some("markdown")
    );
}

#[test]
fn test_markdown_field_normalization() {
    // Create a document with chevrons in both fields
    let mut doc_fields = HashMap::new();
    doc_fields.insert(
        "markdown_field".to_string(),
        QuillValue::from_json(json!("This has <<guillemets>>")),
    );
    doc_fields.insert(
        "string_field".to_string(),
        QuillValue::from_json(json!("This has <<stripped>>")),
    );

    let doc = ParsedDocument::new(doc_fields, QuillReference::latest("test".to_string()));

    // Normalize
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
