//! # API Workflow Tests
//!
//! Tests for the workflow API introduced during API refactoring.
//!
//! ## Purpose
//!
//! This test suite validates the public API for creating and using workflows:
//! - `ParsedDocument::from_markdown()` - Markdown parsing
//! - `Workflow::render()` - Full rendering pipeline
//! - `Workflow::process_plate()` - Template processing only
//!
//! ## Relationship to Other Tests
//!
//! These tests complement `quill_engine_test.rs` by focusing specifically on
//! parsing and high-level rendering workflows. While `quill_engine_test.rs` provides
//! comprehensive integration coverage, these tests validate specific API methods
//! and their contracts.
//!
//! ## Test Strategy
//!
//! - Use minimal quills to reduce test complexity
//! - Focus on API method signatures and behavior
//! - Validate error-free execution paths

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
fn test_render_with_parsed_document() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(
        quill_path.join("plate.typ"),
        "#set page(width: 100pt, height: 100pt)\n= {{ title }}",
    )
    .expect("Failed to write plate.typ");

    let markdown = r#"---
title: My Document
---

# Content

This is the content.
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test-quill")
        .expect("Failed to load workflow");

    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render");

    assert!(!result.artifacts.is_empty());
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Pdf);
}

#[test]
fn test_process_plate() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("plate.typ"), "Title: {{ title }}")
        .expect("Failed to write plate.typ");

    let markdown = r#"---
title: Test Title
---

Some content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test-quill")
        .expect("Failed to load workflow");

    let plated = workflow
        .process_plate(&parsed)
        .expect("Failed to process plate");

    assert!(plated.contains("Test Title"));
}
