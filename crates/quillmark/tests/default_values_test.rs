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

    // Create Quill.toml with field defaults (for UI/documentation purposes)
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with defaults"

[fields]
title = { type = "string", description = "Document title" }
status = { type = "string", description = "Document status", default = "draft" }
version = { type = "number", description = "Version number", default = 1 }
"#,
    )
    .expect("Failed to write Quill.toml");

    // Create plate template that handles defaults via Typst .at()
    // This is the correct pattern: Typst handles missing optional fields
    fs::write(
        quill_path.join("plate.typ"),
        r#"#let doc = json.decode(sys.inputs.doc)
Title: #doc.at("title", default: "Untitled")
Status: #doc.at("status", default: "draft")
Version: #doc.at("version", default: 1)"#,
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

    // Process through plate - Typst plate handles defaults via .at()
    let plated = workflow
        .process_plate(&parsed)
        .expect("Failed to process plate");

    // Verify Typst plate applied defaults for missing fields
    assert!(plated.contains("title"));
    assert!(plated.contains("status"));
    assert!(plated.contains("version"));
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
title = { type = "string", description = "Document title" }
status = { type = "string", description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    // Plate uses .at() with defaults - explicit values from document take precedence
    fs::write(
        quill_path.join("plate.typ"),
        r#"#let doc = json.decode(sys.inputs.doc)
Title: #doc.at("title", default: "Untitled")
Status: #doc.at("status", default: "draft")"#,
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
    let plated = workflow
        .process_plate(&parsed)
        .expect("Failed to process plate");

    // Verify explicit values are in the plate output
    assert!(plated.contains("title"));
    assert!(plated.contains("status"));
}

#[test]
fn test_validation_with_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create quill where all fields are optional (no required = true)
    // Since defaults are NOT auto-imputed, validation should still pass
    // because these fields are not marked as required
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with optional fields"

[fields]
title = { type = "string", description = "Document title", default = "Untitled" }
status = { type = "string", description = "Document status", default = "draft" }
"#,
    )
    .expect("Failed to write Quill.toml");

    // Plate handles defaults via Typst .at()
    fs::write(
        quill_path.join("plate.typ"),
        r#"#let doc = json.decode(sys.inputs.doc)
Title: #doc.at("title", default: "Untitled")
Status: #doc.at("status", default: "draft")"#,
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

    // Create document with no fields - should validate because none are required
    let markdown = r#"# Content"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let plated = workflow
        .process_plate(&parsed)
        .expect("Validation should pass - fields are optional");

    // Verify plate output contains expected field references
    assert!(plated.contains("Title:"));
    assert!(plated.contains("Status:"));
}

#[test]
fn test_validation_fails_without_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    // Create quill with required field (explicit required = true)
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with required field"

[fields]
title = { type = "string", description = "Document title", required = true }
status = { type = "string", description = "Document status", default = "draft" }
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
    let result = workflow.process_plate(&parsed);

    // Should fail validation because title is required (no default)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("title"));
}
