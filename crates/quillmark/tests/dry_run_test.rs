//! # Dry Run Validation Tests
//!
//! Tests for the `Workflow::dry_run()` method that validates input
//! without backend compilation.

use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

fn create_test_quill(temp_dir: &TempDir, with_required_field: bool) -> Quill {
    let quill_path = temp_dir.path().join("test_quill");
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    let fields_section = if with_required_field {
        r#"
main:
  fields:
    title:
      type: "string"
      required: true
      description: "Document title"
    author:
      type: "string"
      required: false
      description: "Document author"
"#
    } else {
        ""
    };

    fs::write(
        quill_path.join("Quill.yaml"),
        format!(
            r#"Quill:
  name: "test_quill"
  version: "1.0"
  backend: "typst"
  plate_file: "plate.typ"
  description: "Test quill"

{}
"#,
            fields_section.replace("=", ":"),
        ),
    )
    .expect("Failed to write Quill.yaml");

    fs::write(quill_path.join("plate.typ"), "Title: {{ title }}")
        .expect("Failed to write plate.typ");

    Quill::from_path(quill_path).expect("Failed to load quill")
}

#[test]
fn test_dry_run_success() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill = create_test_quill(&temp_dir, true);

    let mut engine = Quillmark::new();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test_quill")
        .expect("Failed to load workflow");

    let markdown = r#"---
QUILL: test_quill
title: My Document
author: Test Author
---

# Content

This is the content.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // dry_run should succeed for valid input
    let result = workflow.dry_run(&parsed);
    assert!(result.is_ok(), "dry_run should succeed for valid input");
}

#[test]
fn test_dry_run_missing_required_field() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill = create_test_quill(&temp_dir, true);

    let mut engine = Quillmark::new();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test_quill")
        .expect("Failed to load workflow");

    // Missing required 'title' field
    let markdown = r#"---
QUILL: test_quill
author: Test Author
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // dry_run should fail for missing required field
    let result = workflow.dry_run(&parsed);
    assert!(
        result.is_err(),
        "dry_run should fail for missing required field"
    );

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("ValidationFailed") || err_str.contains("title"),
        "Error should indicate validation failure: {}",
        err_str
    );
}

#[test]
fn test_dry_run_no_schema() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill = create_test_quill(&temp_dir, false); // No required fields

    let mut engine = Quillmark::new();
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test_quill")
        .expect("Failed to load workflow");

    // Any fields should work when there's no schema
    let markdown = r#"---
QUILL: test_quill
random_field: anything
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // dry_run should succeed when no schema is defined
    let result = workflow.dry_run(&parsed);
    assert!(result.is_ok(), "dry_run should succeed without schema");
}
