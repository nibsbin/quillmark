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
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create Quill.toml with field defaults
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with defaults"

[fields]
title = { description = "Document title" }
status = { description = "Document status", default = "draft" }
version = { description = "Version number", default = 1 }
"#,
    )
    .expect("Failed to write Quill.toml");

    // Create plate template that uses default fields
    fs::write(
        quill_path.join("plate.typ"),
        "Title: {{ title }}\nStatus: {{ status }}\nVersion: {{ version }}",
    )
    .expect("Failed to write plate.typ");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test-quill")
        .expect("Failed to load workflow");

    // Create document with only title (missing status and version)
    let markdown = r#"---
title: My Document
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Render plate - defaults should be applied
    let print_output = workflow
        .render_plate(&parsed)
        .expect("Failed to render plate");

    // Verify defaults were applied in the output
    assert!(print_output.contains("Title: My Document"));
    assert!(print_output.contains("Status: draft"));
    assert!(print_output.contains("Version: 1"));
}

#[test]
fn test_default_values_not_overriding_existing_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with defaults"

[fields]
title = { description = "Document title" }
status = { description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    fs::write(
        quill_path.join("plate.typ"),
        "Title: {{ title }}\nStatus: {{ status }}",
    )
    .expect("Failed to write plate.typ");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test-quill")
        .expect("Failed to load workflow");

    // Create document with explicit status value
    let markdown = r#"---
title: My Document
status: published
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let print_output = workflow
        .render_plate(&parsed)
        .expect("Failed to render plate");

    // Verify existing value was preserved, not replaced with default
    assert!(print_output.contains("Title: My Document"));
    assert!(print_output.contains("Status: published"));
    assert!(!print_output.contains("Status: draft"));
}

#[test]
fn test_validation_with_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create quill where all fields have defaults - validation should pass with empty doc
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with defaults"

[fields]
title = { description = "Document title", default = "Untitled" }
status = { description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    fs::write(
        quill_path.join("plate.typ"),
        "Title: {{ title }}\nStatus: {{ status }}",
    )
    .expect("Failed to write plate.typ");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test-quill")
        .expect("Failed to load workflow");

    // Create document with no fields - should validate because all have defaults
    let markdown = r#"# Content"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let print_output = workflow
        .render_plate(&parsed)
        .expect("Validation should pass with defaults");

    // Verify defaults were applied
    assert!(print_output.contains("Title: Untitled"));
    assert!(print_output.contains("Status: draft"));
}

#[test]
fn test_validation_fails_without_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create quill with required field (no default)
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with required field"

[fields]
title = { description = "Document title" }
status = { description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    fs::write(
        quill_path.join("plate.typ"),
        "Title: {{ title }}\nStatus: {{ status }}",
    )
    .expect("Failed to write plate.typ");

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test-quill")
        .expect("Failed to load workflow");

    // Create document missing required title field
    let markdown = r#"---
status: published
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let result = workflow.render_plate(&parsed);

    // Should fail validation because title is required (no default)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("title"));
}
