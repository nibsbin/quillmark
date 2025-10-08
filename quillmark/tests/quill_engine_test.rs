use std::fs;
use tempfile::TempDir;

use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};

#[test]
fn test_quill_engine_creation() {
    let engine = Quillmark::new();

    // Check that typst backend is auto-registered (default feature)
    let backends = engine.registered_backends();
    assert!(backends.contains(&"typst"));
    assert_eq!(backends.len(), 1);

    // Check that no quills are registered initially
    let quills = engine.registered_quills();
    assert_eq!(quills.len(), 0);
}

#[test]
fn test_quill_engine_register_quill() {
    let mut engine = Quillmark::new();

    // Create a test quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "Test template").expect("Failed to write glue.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill);

    // Check that quill is registered
    let quills = engine.registered_quills();
    assert_eq!(quills.len(), 1);
    assert!(quills.contains(&"my-test-quill"));
}

#[test]
fn test_quill_engine_get_workflow() {
    let mut engine = Quillmark::new();

    // Create and register a test quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("glue.typ"),
        "= {{ title | String(default=\"Test\") }}\n\n{{ body | Content }}",
    )
    .expect("Failed to write glue.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill);

    // Load workflow by quill name using new load() method
    let workflow = engine
        .load("my-test-quill")
        .expect("Failed to load workflow");

    // Verify workflow properties
    assert_eq!(workflow.quill_name(), "my-test-quill");
    assert_eq!(workflow.backend_id(), "typst");
    assert!(workflow.supported_formats().contains(&OutputFormat::Pdf));
}

#[test]
fn test_quill_engine_workflow_not_found() {
    let engine = Quillmark::new();

    // Try to load workflow for non-existent quill
    let result = engine.load("non-existent");

    assert!(result.is_err());
    match result {
        Err(quillmark::RenderError::Other(e)) => {
            assert!(e.to_string().contains("not registered"));
        }
        _ => panic!("Expected Other error with 'not registered' message"),
    }
}

#[test]
fn test_quill_engine_backend_not_found() {
    let mut engine = Quillmark::new();

    // Create a quill with non-existent backend
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"bad-backend-quill\"\nbackend = \"non-existent\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "Test template").expect("Failed to write glue.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill);

    // Try to load workflow with non-existent backend
    let result = engine.load("bad-backend-quill");

    assert!(result.is_err());
    match result {
        Err(quillmark::RenderError::Other(e)) => {
            assert!(
                e.to_string().contains("not registered") || e.to_string().contains("not enabled")
            );
        }
        _ => panic!("Expected Other error with backend not registered message"),
    }
}

#[test]
fn test_quill_engine_end_to_end() {
    let mut engine = Quillmark::new();

    // Create and register a test quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");

    let glue_template = r#"
= {{ title | String(default="Untitled") }}

_By {{ author | String(default="Unknown") }}_

{{ body | Content }}
"#;
    fs::write(quill_path.join("glue.typ"), glue_template).expect("Failed to write glue.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill);

    // Load workflow and render
    let workflow = engine
        .load("my-test-quill")
        .expect("Failed to load workflow");

    let markdown = r#"---
title: Test Document
author: Test Author
---

# Introduction

This is a test document with some **bold** text.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render");

    assert!(!result.artifacts.is_empty());
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Pdf);
    assert!(!result.artifacts[0].bytes.is_empty());
}

#[test]
fn test_quill_engine_load_with_quill_object() {
    let mut engine = Quillmark::new();

    // Create a test quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("glue.typ"),
        "= {{ title | String(default=\"Test\") }}\n\n{{ body | Content }}",
    )
    .expect("Failed to write glue.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill.clone());

    // Load workflow by passing Quill object directly
    let workflow = engine.load(&quill).expect("Failed to load workflow");

    // Verify workflow properties
    assert_eq!(workflow.quill_name(), "my-test-quill");
    assert_eq!(workflow.backend_id(), "typst");
    assert!(workflow.supported_formats().contains(&OutputFormat::Pdf));
}

#[test]
fn test_quill_engine_load_with_different_string_types() {
    let mut engine = Quillmark::new();

    // Create and register a test quill
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("glue.typ"),
        "= {{ title | String(default=\"Test\") }}\n\n{{ body | Content }}",
    )
    .expect("Failed to write glue.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill);

    // Test with &str
    let workflow1 = engine
        .load("my-test-quill")
        .expect("Failed to load with &str");
    assert_eq!(workflow1.quill_name(), "my-test-quill");

    // Test with &String
    let quill_name = String::from("my-test-quill");
    let workflow2 = engine
        .load(&quill_name)
        .expect("Failed to load with &String");
    assert_eq!(workflow2.quill_name(), "my-test-quill");
}
