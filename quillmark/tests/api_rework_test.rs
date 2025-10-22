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
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "{{ title }}").expect("Failed to write glue.typ");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow_from_quill_name("test-quill")
        .expect("Failed to load workflow");

    assert_eq!(workflow.quill_name(), "test-quill");
}

#[test]
fn test_workflow_from_quill() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("glue.typ"), "{{ title }}").expect("Failed to write glue.typ");

    let engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    let workflow = engine
        .workflow_from_quill(&quill)
        .expect("Failed to load workflow");

    assert_eq!(workflow.quill_name(), "test-quill");
}

#[test]
fn test_render_with_parsed_document() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"\n",
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
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow_from_quill_name("test-quill")
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
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"test-quill\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"\n",
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
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow_from_quill_name("test-quill")
        .expect("Failed to load workflow");

    let glue = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    assert!(glue.contains("Test Title"));
}
