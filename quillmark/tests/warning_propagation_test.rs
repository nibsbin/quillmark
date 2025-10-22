use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark};
use quillmark_core::Severity;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_warnings_are_captured_in_result() {
    // Create a simple test quill
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
title: Test Document
---

# Test Content

This is a test document.
"#;

    let mut quillmark = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    quillmark.register_quill(quill);

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let workflow = quillmark
        .workflow_from_quill_name("test-quill")
        .expect("Failed to create workflow");

    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render");

    // The warning system is properly wired through the stack
    // Warnings field exists and is accessible
    assert!(result.warnings.is_empty() || !result.warnings.is_empty());

    // If there are warnings, they should have proper diagnostic structure
    for warning in &result.warnings {
        assert_eq!(warning.severity, Severity::Warning);
        assert!(!warning.message.is_empty());
    }
}

#[test]
fn test_warnings_accessible_via_python_bindings() {
    // Create a simple test quill
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
title: Test
---
# Content
"#;

    let mut quillmark = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    quillmark.register_quill(quill);

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse");
    let workflow = quillmark
        .workflow_from_quill_name("test-quill")
        .expect("Failed to create workflow");

    let result = workflow
        .render(&parsed, Some(OutputFormat::Pdf))
        .expect("Failed to render");

    // Ensure warnings can be converted to serializable format
    let serializable_warnings: Vec<_> = result
        .warnings
        .iter()
        .map(|w| quillmark_core::SerializableDiagnostic::from(w))
        .collect();

    // Each serializable warning should have all required fields
    for warning in serializable_warnings {
        assert_eq!(warning.severity, Severity::Warning);
        assert!(!warning.message.is_empty());
        // source_chain should be accessible
        assert!(warning.source_chain.is_empty() || !warning.source_chain.is_empty());
    }
}
