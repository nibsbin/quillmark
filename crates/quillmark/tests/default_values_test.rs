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
//! - **Validation with defaults** - Required field validation behavior
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
//! When rendering, missing fields are populated with defaults before
//! JSON serialization, ensuring plates always have expected values.
//!
//! ## Design Reference
//!
//! See `prose/designs/SCHEMAS.md` for field schema specification.

use quillmark::{ParsedDocument, Quill, Quillmark};
use std::fs;
use tempfile::TempDir;

/// Helper to create a test quill
fn create_test_quill(temp_dir: &TempDir, quill_toml: &str) -> std::path::PathBuf {
    let quill_path = temp_dir.path().join("test_quill");
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");

    fs::write(quill_path.join("Quill.toml"), quill_toml).expect("Failed to write Quill.toml");

    // Create a minimal plate template
    fs::write(
        quill_path.join("plate.typ"),
        r#"#import "@local/quillmark-helper:0.1.0": data
= Document
#data"#,
    )
    .expect("Failed to write plate.typ");

    quill_path
}

#[test]
fn test_default_values_applied_via_dry_run() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let quill_path = create_test_quill(
        &temp_dir,
        r#"[Quill]
name = "test_quill"
version = "1.0"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with defaults"

[fields]
title = { type = "string", description = "Document title" }
status = { type = "string", description = "Document status", default = "draft" }
version = { type = "number", description = "Version number", default = 1 }
"#,
    );

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test_quill")
        .expect("Failed to load workflow");

    // Create document with only title (missing status and version)
    let markdown = r#"---
title: My Document
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // dry_run validates without backend compilation - should succeed
    // because missing optional fields have defaults
    let result = workflow.dry_run(&parsed);
    assert!(
        result.is_ok(),
        "Dry run should succeed - optional fields have defaults"
    );
}

#[test]
fn test_default_values_not_overriding_existing_fields() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let quill_path = create_test_quill(
        &temp_dir,
        r#"[Quill]
name = "test_quill"
version = "1.0"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with defaults"

[fields]
title = { type = "string", description = "Document title" }
status = { type = "string", description = "Document status", default = "draft" }
"#,
    );

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test_quill")
        .expect("Failed to load workflow");

    // Create document with explicit status value
    let markdown = r#"---
title: My Document
status: published
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // dry_run should succeed - explicit values take precedence over defaults
    let result = workflow.dry_run(&parsed);
    assert!(
        result.is_ok(),
        "Dry run should succeed with explicit values"
    );
}

#[test]
fn test_validation_with_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let quill_path = create_test_quill(
        &temp_dir,
        r#"[Quill]
name = "test_quill"
version = "1.0"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with optional fields"

[fields]
title = { type = "string", description = "Document title", default = "Untitled" }
status = { type = "string", description = "Document status", default = "draft" }
"#,
    );

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test_quill")
        .expect("Failed to load workflow");

    // Create document with no fields - should validate because none are required
    // and defaults will be applied
    let markdown = r#"# Content"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // dry_run validates without rendering - should pass with defaults
    let dry_run_result = workflow.dry_run(&parsed);
    assert!(
        dry_run_result.is_ok(),
        "Dry run should pass - fields have defaults"
    );
}

#[test]
fn test_validation_fails_without_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let quill_path = create_test_quill(
        &temp_dir,
        r#"[Quill]
name = "test_quill"
version = "1.0"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill with required field"

[fields]
title = { type = "string", description = "Document title", required = true }
status = { type = "string", description = "Document status", default = "draft" }
"#,
    );

    let mut engine = Quillmark::new();
    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    let workflow = engine
        .workflow("test_quill")
        .expect("Failed to load workflow");

    // Create document missing required title field
    let markdown = r#"---
status: published
---

# Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // dry_run should fail because title is required (no default)
    let result = workflow.dry_run(&parsed);
    assert!(
        result.is_err(),
        "Should fail validation - title is required"
    );

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("title"),
        "Error should mention missing 'title' field"
    );
}

#[test]
fn test_extract_defaults_from_quill() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test_quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        r#"[Quill]
name = "test_quill"
version = "1.0"
backend = "typst"
description = "Test"

[fields]
author = { type = "string", default = "Anonymous" }
priority = { type = "number", default = 5 }
draft = { type = "boolean", default = true }
"#,
    )
    .expect("Failed to write Quill.toml");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");

    // Verify extract_defaults returns the schema defaults
    let defaults = quill.extract_defaults();

    assert!(defaults.contains_key("author"));
    assert_eq!(defaults.get("author").unwrap().as_str(), Some("Anonymous"));

    assert!(defaults.contains_key("priority"));
    assert_eq!(defaults.get("priority").unwrap().as_i64(), Some(5));

    assert!(defaults.contains_key("draft"));
    assert_eq!(defaults.get("draft").unwrap().as_bool(), Some(true));
}
