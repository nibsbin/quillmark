use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_default_values_applied_in_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("default-test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create Quill.toml with fields that have defaults
    let quill_toml = r#"[Quill]
name = "default-test-quill"
backend = "typst"
glue_file = "glue.typ"
description = "Test quill for default values"

[fields]
title = { description = "Document title", type = "str" }
status = { description = "Document status", type = "str", default = "draft" }
version = { description = "Document version", type = "number", default = 1 }
tags = { description = "Document tags", type = "array", default = [] }
"#;

    fs::write(quill_path.join("Quill.toml"), quill_toml).expect("Failed to write Quill.toml");

    // Create a simple glue template that uses all fields
    let glue = r#"= {{ title }}

Status: {{ status }}
Version: {{ version }}
Tags: {{ tags }}
"#;
    fs::write(quill_path.join("glue.typ"), glue).expect("Failed to write glue.typ");

    // Create engine and register quill
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Create a document that only provides "title" (other fields should get defaults)
    let markdown = r#"---
title: "My Document"
---

Content here.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Create workflow
    let workflow = engine
        .workflow_from_quill_name("default-test-quill")
        .expect("Failed to create workflow");

    // Validate should succeed because defaults are applied
    workflow
        .validate(&parsed)
        .expect("Validation should succeed with defaults applied");

    // Process glue to verify defaults are used in template
    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    // Verify the output contains the default values
    assert!(glue_output.contains("Status: draft"));
    assert!(glue_output.contains("Version: 1"));
}

#[test]
fn test_default_values_not_override_existing() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("override-test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create Quill.toml with a field that has a default
    let quill_toml = r#"[Quill]
name = "override-test-quill"
backend = "typst"
glue_file = "glue.typ"
description = "Test quill for override behavior"

[fields]
title = { description = "Document title", type = "str" }
status = { description = "Document status", type = "str", default = "draft" }
"#;

    fs::write(quill_path.join("Quill.toml"), quill_toml).expect("Failed to write Quill.toml");

    let glue = "Status: {{ status }}";
    fs::write(quill_path.join("glue.typ"), glue).expect("Failed to write glue.typ");

    // Create engine and register quill
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Create a document that provides an explicit value for "status"
    let markdown = r#"---
title: "My Document"
status: "published"
---

Content here.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Create workflow
    let workflow = engine
        .workflow_from_quill_name("override-test-quill")
        .expect("Failed to create workflow");

    // Process glue
    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    // Verify the output uses the explicit value, not the default
    assert!(glue_output.contains("Status: published"));
    assert!(!glue_output.contains("Status: draft"));
}

#[test]
fn test_missing_required_field_without_default_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("required-test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create Quill.toml with required field (no default)
    let quill_toml = r#"[Quill]
name = "required-test-quill"
backend = "typst"
glue_file = "glue.typ"
description = "Test quill for required fields"

[fields]
title = { description = "Document title", type = "str" }
author = { description = "Document author", type = "str" }
"#;

    fs::write(quill_path.join("Quill.toml"), quill_toml).expect("Failed to write Quill.toml");

    let glue = "{{ title }} by {{ author }}";
    fs::write(quill_path.join("glue.typ"), glue).expect("Failed to write glue.typ");

    // Create engine and register quill
    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Create a document missing "author" (required field without default)
    let markdown = r#"---
title: "My Document"
---

Content here.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Create workflow
    let workflow = engine
        .workflow_from_quill_name("required-test-quill")
        .expect("Failed to create workflow");

    // Validation should fail because "author" is required and not provided
    let result = workflow.validate(&parsed);
    assert!(
        result.is_err(),
        "Validation should fail for missing required field"
    );
}
