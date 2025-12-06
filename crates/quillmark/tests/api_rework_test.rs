//! # API Workflow Tests
//!
//! Tests for the workflow API introduced during API refactoring.
//!
//! ## Purpose
//!
//! This test suite validates the public API for creating and using workflows:
//! - `ParsedDocument::from_markdown()` - Markdown parsing
//! - `Quillmark::workflow_from_quill_name()` - Load workflow by plate name
//! - `Quillmark::workflow_from_quill()` - Load workflow from plate object
//! - `Workflow::render()` - Full rendering pipeline
//! - `Workflow::process_glue()` - Template processing only
//!
//! ## Relationship to Other Tests
//!
//! These tests complement `quill_engine_test.rs` by focusing specifically on
//! workflow creation and usage patterns. While `quill_engine_test.rs` provides
//! comprehensive integration coverage, these tests validate specific API methods
//! and their contracts.
//!
//! ## Test Strategy
//!
//! - Use minimal quills to reduce test complexity
//! - Focus on API method signatures and behavior
//! - Validate error-free execution paths
//! - Tests intentionally overlap with `quill_engine_test.rs` for redundancy

use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_parsed_document_from_markdown() {
    let markdown = r#"---
title: Test Document
author: John Doe
---

# Hello World

This is a test.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    assert_eq!(
        parsed.get_field("title").and_then(|v| v.as_str()),
        Some("Test Document")
    );
    assert_eq!(
        parsed.get_field("author").and_then(|v| v.as_str()),
        Some("John Doe")
    );
    assert!(parsed.body().is_some());
}

#[test]
fn test_workflow_from_quill_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-plate\"\nbackend = \"typst\"\nglue_file = \"glue.typ\"\ndescription = \"Test plate\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "{{ title }}").expect("Failed to write glue.typ");

    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");
    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    let workflow = engine
        .workflow("test-plate")
        .expect("Failed to load workflow");

    assert_eq!(workflow.plate_name(), "test-plate");
}

#[test]
fn test_workflow_from_quill() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-plate\"\nbackend = \"typst\"\nglue_file = \"glue.typ\"\ndescription = \"Test plate\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "{{ title }}").expect("Failed to write glue.typ");

    let engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");

    let workflow = engine.workflow(&plate).expect("Failed to load workflow");

    assert_eq!(workflow.plate_name(), "test-plate");
}

#[test]
fn test_render_with_parsed_document() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-plate\"\nbackend = \"typst\"\nglue_file = \"glue.typ\"\ndescription = \"Test plate\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("glue.typ"),
        "#set page(width: 100pt, height: 100pt)\n= {{ title }}",
    )
    .expect("Failed to write glue.typ");

    let markdown = r#"---
title: My Document
---

# Content

This is the content.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");
    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    let workflow = engine
        .workflow("test-plate")
        .expect("Failed to load workflow");

    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render");

    assert!(!result.artifacts.is_empty());
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Pdf);
}

#[test]
fn test_process_glue() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-plate\"\nbackend = \"typst\"\nglue_file = \"glue.typ\"\ndescription = \"Test plate\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "Title: {{ title }}").expect("Failed to write glue.typ");

    let markdown = r#"---
title: Test Title
---

Some content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");
    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    let workflow = engine
        .workflow("test-plate")
        .expect("Failed to load workflow");

    let glue = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    assert!(glue.contains("Test Title"));
}
