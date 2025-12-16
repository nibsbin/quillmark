//! # Quillmark Engine Integration Tests
//!
//! Comprehensive integration tests for the `Quillmark` engine and `Workflow` orchestration.
//!
//! ## Test Coverage
//!
//! This test suite validates:
//! - **Engine creation and initialization** - Backend auto-registration, default quill setup
//! - **Quill registration** - Custom quill loading and management
//! - **Workflow creation** - Loading workflows by name, by quill object, and from parsed documents
//! - **End-to-end rendering** - Complete parse → template → compile pipeline
//! - **Error handling** - Missing quills, invalid backends, validation failures
//! - **API ergonomics** - Different string types, QuillRef patterns
//!
//! ## Related Tests
//!
//! - `api_rework_test.rs` - Focused API validation for new workflow methods
//! - `backend_registration_test.rs` - Custom backend registration scenarios
//! - `default_quill_test.rs` - Default quill system behavior
//!
//! ## Test Philosophy
//!
//! These tests use temporary directories and create custom quills to validate
//! the full integration of the engine. They complement unit tests in individual
//! crates by exercising the complete public API surface.

use std::fs;
use tempfile::TempDir;

use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};

#[test]
fn test_quill_engine_creation() {
    let engine = Quillmark::new();

    // Check that at least one backend is registered (if default features enabled)
    let backends = engine.registered_backends();
    #[cfg(any(feature = "typst", feature = "acroform"))]
    assert!(!backends.is_empty());

    // Check that default quill is registered when typst backend is enabled
    let quills = engine.registered_quills();
    #[cfg(feature = "typst")]
    assert_eq!(quills.len(), 1);
    #[cfg(feature = "typst")]
    assert!(quills.contains(&"__default__"));
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
        "[Quill]\nname = \"my_test_quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("plate.typ"), "Test template").expect("Failed to write plate.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Check that quill is registered (plus __default__)
    let quills = engine.registered_quills();
    #[cfg(feature = "typst")]
    assert_eq!(quills.len(), 2); // __default__ + my_test_quill
    #[cfg(not(feature = "typst"))]
    assert_eq!(quills.len(), 1); // just my_test_quill
    assert!(quills.contains(&"my_test_quill"));
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
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("plate.typ"),
        "= {{ title | String(default=\"Test\") }}\n\n{{ body | Content }}",
    )
    .expect("Failed to write plate.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Load workflow by quill name using new load() method
    let workflow = engine
        .workflow("my-test-quill")
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
    let result = engine.workflow("non-existent");

    assert!(result.is_err());
    match result {
        Err(quillmark::RenderError::UnsupportedBackend { diag }) => {
            assert!(diag.message.contains("not registered"));
        }
        _ => panic!("Expected UnsupportedBackend error with 'not registered' message"),
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
        "[Quill]\nname = \"bad-backend-quill\"\nbackend = \"non-existent\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("plate.typ"), "Test template").expect("Failed to write plate.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // Try to register quill with non-existent backend - should fail now
    let result = engine.register_quill(quill);

    assert!(result.is_err());
    match result {
        Err(quillmark::RenderError::QuillConfig { diag }) => {
            assert!(diag.message.contains("not registered"));
            assert!(diag.code == Some("quill::backend_not_found".to_string()));
        }
        _ => panic!("Expected QuillConfig error with backend not registered message"),
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
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");

    let plate_template = r#"
= {{ title | String(default="Untitled") }}

_By {{ author | String(default="Unknown") }}_

{{ body | Content }}
"#;
    fs::write(quill_path.join("plate.typ"), plate_template).expect("Failed to write plate.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Load workflow and render
    let workflow = engine
        .workflow("my-test-quill")
        .expect("Failed to load workflow");

    let markdown = r#"---
title: Test Document
author: Test Author
---

# Introduction

This is a test document with some **bold** text.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let plated = workflow
        .process_plate(&parsed)
        .expect("Failed to process plate");

    println!("DEBUG: Plated content:\n{}", plated);

    let result = workflow
        .render_plate(&plated, Some(OutputFormat::Pdf))
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
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("plate.typ"),
        "= {{ title | String(default=\"Test\") }}\n\n{{ body | Content }}",
    )
    .expect("Failed to write plate.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill.clone())
        .expect("Failed to register quill");

    // Load workflow by passing Quill object directly
    let workflow = engine.workflow(&quill).expect("Failed to load workflow");

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
        "[Quill]\nname = \"my-test-quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("plate.typ"),
        "= {{ title | String(default=\"Test\") }}\n\n{{ body | Content }}",
    )
    .expect("Failed to write plate.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Test with &str
    let workflow1 = engine
        .workflow("my-test-quill")
        .expect("Failed to load with &str");
    assert_eq!(workflow1.quill_name(), "my-test-quill");

    // Test with &String
    let quill_name = String::from("my-test-quill");
    let workflow2 = engine
        .workflow(&quill_name)
        .expect("Failed to load with &String");
    assert_eq!(workflow2.quill_name(), "my-test-quill");
}
