//! Integration tests for Quill.fromFiles functionality

use quillmark_core::{FileEntry, Quill};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_quill_from_files_manual() {
    // Simulate what the WASM layer does - create a Quill from in-memory files
    let mut files = HashMap::new();

    // Add Quill.toml
    let quill_toml = r#"
[Quill]
name = "test_quill"
backend = "typst"
glue = "glue.typ"
template = "template.md"
description = "A test quill"
author = "Test Author"
"#;
    files.insert(
        PathBuf::from("Quill.toml"),
        FileEntry {
            contents: quill_toml.as_bytes().to_vec(),
            path: PathBuf::from("Quill.toml"),
            is_dir: false,
        },
    );

    // Add glue template
    let glue_content = r#"
#let render(doc) = {
  doc.body
}
"#;
    files.insert(
        PathBuf::from("glue.typ"),
        FileEntry {
            contents: glue_content.as_bytes().to_vec(),
            path: PathBuf::from("glue.typ"),
            is_dir: false,
        },
    );

    // Add markdown template
    let template_content = r#"
# {{ title }}

{{ body }}
"#;
    files.insert(
        PathBuf::from("template.md"),
        FileEntry {
            contents: template_content.as_bytes().to_vec(),
            path: PathBuf::from("template.md"),
            is_dir: false,
        },
    );

    // Create metadata
    let mut metadata = HashMap::new();
    metadata.insert(
        "backend".to_string(),
        serde_yaml::Value::String("typst".to_string()),
    );
    metadata.insert(
        "description".to_string(),
        serde_yaml::Value::String("A test quill".to_string()),
    );
    metadata.insert(
        "author".to_string(),
        serde_yaml::Value::String("Test Author".to_string()),
    );

    // Create the Quill manually
    let quill = Quill {
        glue_template: glue_content.to_string(),
        metadata,
        base_path: PathBuf::from("/"),
        name: "test_quill".to_string(),
        glue_file: "glue.typ".to_string(),
        template_file: Some("template.md".to_string()),
        template: Some(template_content.to_string()),
        files,
    };

    // Validate it
    let result = quill.validate();
    assert!(
        result.is_ok(),
        "Quill validation should succeed: {:?}",
        result
    );

    // Check metadata
    assert_eq!(quill.name, "test_quill");
    assert_eq!(
        quill.metadata.get("backend").and_then(|v| v.as_str()),
        Some("typst")
    );
    assert_eq!(
        quill.metadata.get("description").and_then(|v| v.as_str()),
        Some("A test quill")
    );
    assert_eq!(
        quill.metadata.get("author").and_then(|v| v.as_str()),
        Some("Test Author")
    );

    // Check files
    assert_eq!(quill.files.len(), 3);
    assert!(quill.files.contains_key(&PathBuf::from("Quill.toml")));
    assert!(quill.files.contains_key(&PathBuf::from("glue.typ")));
    assert!(quill.files.contains_key(&PathBuf::from("template.md")));
}
