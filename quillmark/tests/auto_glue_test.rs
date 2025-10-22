use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_auto_glue_without_glue_file() {
    // Create a quill without a glue file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("auto-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"auto-quill\"\nbackend = \"typst\"\ndescription = \"Test auto glue\"\n",
    )
    .expect("Failed to write Quill.toml");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // Verify glue_file is None
    assert_eq!(quill.metadata.get("glue_file").and_then(|v| v.as_str()), None);
    assert_eq!(quill.glue.clone().unwrap_or_default(), "");

    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow_from_quill_name("auto-quill")
        .expect("Failed to load workflow");

    assert_eq!(workflow.quill_name(), "auto-quill");
}

#[test]
fn test_auto_glue_output() {
    // Create a quill without a glue file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("auto-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"auto-quill\"\nbackend = \"typst\"\ndescription = \"Test auto glue\"\n",
    )
    .expect("Failed to write Quill.toml");

    let markdown = r#"---
title: Test Document
author: Alice Smith
tags:
  - markdown
  - json
  - test
---

# Hello World

This is a test document.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow_from_quill_name("auto-quill")
        .expect("Failed to load workflow");

    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    // Parse the output as JSON
    let json: serde_json::Value =
        serde_json::from_str(&glue_output).expect("Output is not valid JSON");

    // Verify the structure
    assert_eq!(json["title"], "Test Document");
    assert_eq!(json["author"], "Alice Smith");
    assert!(json["tags"].is_array());
    assert_eq!(json["tags"].as_array().unwrap().len(), 3);
    assert_eq!(json["tags"][0], "markdown");
    assert!(json["body"].is_string());
}

#[test]
fn test_auto_glue_with_nested_data() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("auto-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"auto-quill\"\nbackend = \"typst\"\ndescription = \"Test auto glue\"\n",
    )
    .expect("Failed to write Quill.toml");

    let markdown = r#"---
metadata:
  version: "1.0"
  status: draft
contact:
  email: test@example.com
  phone: "555-1234"
---

Content here.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow_from_quill_name("auto-quill")
        .expect("Failed to load workflow");

    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    // Parse the output as JSON
    let json: serde_json::Value =
        serde_json::from_str(&glue_output).expect("Output is not valid JSON");

    // Verify nested structure
    assert_eq!(json["metadata"]["version"], "1.0");
    assert_eq!(json["metadata"]["status"], "draft");
    assert_eq!(json["contact"]["email"], "test@example.com");
    assert_eq!(json["contact"]["phone"], "555-1234");
}