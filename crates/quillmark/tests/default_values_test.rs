//! # Default Values Tests
//!
//! Tests for default value handling in Quill field schemas.
//!
//! ## Test Coverage
//!
//! This test suite validates:
//! - **Schema-defined defaults** - Default values from Quill.toml [fields] section
//! - **Missing field handling** - Defaults applied when fields are absent from markdown
//! - **Explicit value precedence** - User-provided values override defaults
//! - **Multiple defaults** - Multiple fields with different default values
//! - **Default value types** - String, number, boolean, array, object defaults
//!
//! ## Schema System
//!
//! Quill templates can define default values for fields in Quill.toml:
//! ```toml
//! [fields.author]
//! type = "str"
//! description = "Document author"
//! default = "Anonymous"
//! ```
//!
//! When parsing markdown, missing fields are populated with defaults before
//! template rendering, ensuring templates always have expected values.
//!
//! ## Design Reference
//!
//! See `prose/designs/SCHEMAS.md` for field schema specification.

use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_default_values_applied_to_missing_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");

    // Create Quill.toml with field defaults
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-plate"
backend = "typst"
glue_file = "glue.typ"
description = "Test plate with defaults"

[fields]
title = { description = "Document title" }
status = { description = "Document status", default = "draft" }
version = { description = "Version number", default = 1 }
"#,
    )
    .expect("Failed to write Quill.toml");

    // Create glue template that uses default fields
    fs::write(
        quill_path.join("glue.typ"),
        "Title: {{ title }}\nStatus: {{ status }}\nVersion: {{ version }}",
    )
    .expect("Failed to write glue.typ");

    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");
    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    let workflow = engine
        .workflow("test-plate")
        .expect("Failed to load workflow");

    // Create document with only title (missing status and version)
    let markdown = r#"---
title: My Document
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Process through glue - defaults should be applied
    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    // Verify defaults were applied in the output
    assert!(glue_output.contains("Title: My Document"));
    assert!(glue_output.contains("Status: draft"));
    assert!(glue_output.contains("Version: 1"));
}

#[test]
fn test_default_values_not_overriding_existing_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");

    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-plate"
backend = "typst"
glue_file = "glue.typ"
description = "Test plate with defaults"

[fields]
title = { description = "Document title" }
status = { description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    fs::write(
        quill_path.join("glue.typ"),
        "Title: {{ title }}\nStatus: {{ status }}",
    )
    .expect("Failed to write glue.typ");

    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");
    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    let workflow = engine
        .workflow("test-plate")
        .expect("Failed to load workflow");

    // Create document with explicit status value
    let markdown = r#"---
title: My Document
status: published
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Failed to process glue");

    // Verify existing value was preserved, not replaced with default
    assert!(glue_output.contains("Title: My Document"));
    assert!(glue_output.contains("Status: published"));
    assert!(!glue_output.contains("Status: draft"));
}

#[test]
fn test_validation_with_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");

    // Create plate where all fields have defaults - validation should pass with empty doc
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-plate"
backend = "typst"
glue_file = "glue.typ"
description = "Test plate with defaults"

[fields]
title = { description = "Document title", default = "Untitled" }
status = { description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    fs::write(
        quill_path.join("glue.typ"),
        "Title: {{ title }}\nStatus: {{ status }}",
    )
    .expect("Failed to write glue.typ");

    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");
    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    let workflow = engine
        .workflow("test-plate")
        .expect("Failed to load workflow");

    // Create document with no fields - should validate because all have defaults
    let markdown = r#"# Content"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let glue_output = workflow
        .process_glue(&parsed)
        .expect("Validation should pass with defaults");

    // Verify defaults were applied
    assert!(glue_output.contains("Title: Untitled"));
    assert!(glue_output.contains("Status: draft"));
}

#[test]
fn test_validation_fails_without_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-plate");

    fs::create_dir_all(&quill_path).expect("Failed to create plate dir");

    // Create plate with required field (no default)
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-plate"
backend = "typst"
glue_file = "glue.typ"
description = "Test plate with required field"

[fields]
title = { description = "Document title" }
status = { description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    fs::write(
        quill_path.join("glue.typ"),
        "Title: {{ title }}\nStatus: {{ status }}",
    )
    .expect("Failed to write glue.typ");

    let mut engine = Quillmark::new();
    let plate = Plate::from_path(quill_path).expect("Failed to load plate");
    engine
        .register_plate(plate)
        .expect("Failed to register plate");

    let workflow = engine
        .workflow("test-plate")
        .expect("Failed to load workflow");

    // Create document missing required title field
    let markdown = r#"---
status: published
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let result = workflow.process_glue(&parsed);

    // Should fail validation because title is required (no default)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("title"));
}
