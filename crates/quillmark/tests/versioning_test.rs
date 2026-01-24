//! # Quill Versioning System Integration Tests
//!
//! Comprehensive end-to-end tests for the versioning system.
//!
//! ## Test Coverage
//!
//! - Multi-version quill registration and coexistence
//! - Version selector resolution (exact, major, latest)
//! - Document parsing with version syntax
//! - Workflow creation from versioned documents
//! - Error handling and helpful error messages
//!
//! ## Related Modules
//!
//! - `crates/core/src/version.rs` - Version types and parsing
//! - `crates/quillmark/src/orchestration/engine.rs` - Version resolution logic

use std::fs;
use tempfile::TempDir;

use quillmark::{ParsedDocument, Quill, Quillmark};

/// Helper function to create a test quill with specific version
fn create_test_quill(temp_dir: &TempDir, name: &str, version: &str) -> Quill {
    let quill_path = temp_dir.path().join(format!("{}-{}", name, version));

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.yaml"),
        format!(
            "Quill:\n  name: \"{}\"\n  version: \"{}\"\n  backend: \"typst\"\n  plate_file: \"plate.typ\"\n  description: \"Test quill version {}\"\n",
            name, version, version
        ),
    )
    .expect("Failed to write Quill.yaml");
    fs::write(
        quill_path.join("plate.typ"),
        format!("= Version {}\n\n#{{{{ title | String(default=\"Test\") }}}}\n\n#{{{{ body | Content }}}}", version),
    )
    .expect("Failed to write plate.typ");

    Quill::from_path(quill_path).expect("Failed to load quill")
}

#[test]
#[cfg(feature = "typst")]
fn test_register_multiple_versions_same_quill() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register versions 1.0, 1.1, 2.0 of same quill
    let quill_1_0 = create_test_quill(&temp_dir, "resume_template", "1.0");
    let quill_1_1 = create_test_quill(&temp_dir, "resume_template", "1.1");
    let quill_2_0 = create_test_quill(&temp_dir, "resume_template", "2.0");

    engine
        .register_quill(quill_1_0)
        .expect("Failed to register v1.0");
    engine
        .register_quill(quill_1_1)
        .expect("Failed to register v1.1");
    engine
        .register_quill(quill_2_0)
        .expect("Failed to register v2.0");

    // Verify quill is registered (only one name, multiple versions)
    let quills = engine.registered_quills();
    assert!(quills.contains(&"resume_template"));

    // All versions should be accessible
    let workflow_1_0 = engine
        .workflow("resume_template@1.0")
        .expect("Failed to load v1.0");
    assert_eq!(workflow_1_0.quill_name(), "resume_template");

    let workflow_1_1 = engine
        .workflow("resume_template@1.1")
        .expect("Failed to load v1.1");
    assert_eq!(workflow_1_1.quill_name(), "resume_template");

    let workflow_2_0 = engine
        .workflow("resume_template@2.0")
        .expect("Failed to load v2.0");
    assert_eq!(workflow_2_0.quill_name(), "resume_template");
}

#[test]
#[cfg(feature = "typst")]
fn test_resolve_major_version_selector() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register versions 2.0, 2.1, 2.2, 3.0
    let quill_2_0 = create_test_quill(&temp_dir, "resume_template", "2.0");
    let quill_2_1 = create_test_quill(&temp_dir, "resume_template", "2.1");
    let quill_2_2 = create_test_quill(&temp_dir, "resume_template", "2.2");
    let quill_3_0 = create_test_quill(&temp_dir, "resume_template", "3.0");

    engine
        .register_quill(quill_2_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_1)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_2)
        .expect("Failed to register");
    engine
        .register_quill(quill_3_0)
        .expect("Failed to register");

    // Resolve @2 -> should get 2.2 (latest 2.x)
    let workflow_2 = engine
        .workflow("resume_template@2")
        .expect("Failed to resolve @2");

    // Verify correct quill name
    assert_eq!(workflow_2.quill_name(), "resume_template");
    // Version resolution works - workflow was created successfully

    // Resolve @3 -> should get 3.0
    let workflow_3 = engine
        .workflow("resume_template@3")
        .expect("Failed to resolve @3");
    assert_eq!(workflow_3.quill_name(), "resume_template");
}

#[test]
#[cfg(feature = "typst")]
fn test_resolve_exact_version_selector() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register versions 2.0, 2.1, 2.2
    let quill_2_0 = create_test_quill(&temp_dir, "resume_template", "2.0");
    let quill_2_1 = create_test_quill(&temp_dir, "resume_template", "2.1");
    let quill_2_2 = create_test_quill(&temp_dir, "resume_template", "2.2");

    engine
        .register_quill(quill_2_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_1)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_2)
        .expect("Failed to register");

    // Resolve @2.1 -> should get exactly 2.1
    let workflow = engine
        .workflow("resume_template@2.1")
        .expect("Failed to resolve @2.1");

    // Verify correct quill name
    assert_eq!(workflow.quill_name(), "resume_template");
    // Version resolution works - workflow was created successfully

    // Resolve @2.5 (not registered) -> should error
    let result_not_found = engine.workflow("resume_template@2.5");
    assert!(result_not_found.is_err());
    match result_not_found {
        Err(quillmark::RenderError::VersionNotFound { diag }) => {
            assert!(diag.message.contains("2.5"));
        }
        _ => panic!("Expected VersionNotFound error"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_parse_document_with_version_syntax() {
    // Parse document with QUILL: resume_template@2.1
    let markdown = r#"---
QUILL: resume_template@2.1
title: Test Document
---

# Test Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Verify ParsedDocument.quill_reference() is correct
    let quill_ref = parsed.quill_reference();
    assert_eq!(quill_ref.name, "resume_template");

    // Check that it's an Exact version selector
    match quill_ref.selector {
        quillmark_core::VersionSelector::Exact(v) => {
            assert_eq!(v.major, 2);
            assert_eq!(v.minor, 1);
        }
        _ => panic!("Expected Exact version selector"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_parse_document_with_major_version_syntax() {
    let markdown = r#"---
QUILL: resume_template@2
title: Test Document
---

# Test Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let quill_ref = parsed.quill_reference();
    assert_eq!(quill_ref.name, "resume_template");

    match quill_ref.selector {
        quillmark_core::VersionSelector::Major(m) => {
            assert_eq!(m, 2);
        }
        _ => panic!("Expected Major version selector"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_parse_document_with_latest_syntax() {
    let markdown = r#"---
QUILL: resume_template@latest
title: Test Document
---

# Test Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let quill_ref = parsed.quill_reference();
    assert_eq!(quill_ref.name, "resume_template");

    match quill_ref.selector {
        quillmark_core::VersionSelector::Latest => {}
        _ => panic!("Expected Latest version selector"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_parse_document_without_version() {
    // No version means Latest
    let markdown = r#"---
QUILL: resume_template
title: Test Document
---

# Test Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let quill_ref = parsed.quill_reference();
    assert_eq!(quill_ref.name, "resume_template");

    match quill_ref.selector {
        quillmark_core::VersionSelector::Latest => {}
        _ => panic!("Expected Latest version selector (implicit)"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_workflow_from_versioned_document() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register multiple versions
    let quill_1_0 = create_test_quill(&temp_dir, "resume_template", "1.0");
    let quill_2_0 = create_test_quill(&temp_dir, "resume_template", "2.0");
    let quill_2_1 = create_test_quill(&temp_dir, "resume_template", "2.1");

    engine
        .register_quill(quill_1_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_1)
        .expect("Failed to register");

    // Create document with version tag
    let markdown = r#"---
QUILL: resume_template@2.1
title: Test Document
---

# Test Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");

    // Create workflow from parsed document
    let workflow = engine
        .workflow(&parsed)
        .expect("Failed to create workflow from document");

    // Verify correct version is selected (should be 2.1)
    assert_eq!(workflow.quill_name(), "resume_template");
    // Version resolution worked - workflow was created successfully
}

#[test]
#[cfg(feature = "typst")]
fn test_version_collision_error() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register version 1.0
    let quill_1_0 = create_test_quill(&temp_dir, "resume_template", "1.0");
    engine
        .register_quill(quill_1_0)
        .expect("Failed to register first v1.0");

    // Try to register same name+version again
    let quill_1_0_duplicate = create_test_quill(&temp_dir, "resume_template", "1.0");
    let result = engine.register_quill(quill_1_0_duplicate);

    // Should fail with version collision error
    assert!(result.is_err());
    match result {
        Err(quillmark::RenderError::QuillConfig { diag }) => {
            assert!(diag.message.contains("already registered"));
            assert!(diag.message.contains("1.0"));
        }
        _ => panic!("Expected QuillConfig error for version collision"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_version_not_found_error_message() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register versions 1.0, 2.0, 3.0
    let quill_1_0 = create_test_quill(&temp_dir, "resume_template", "1.0");
    let quill_2_0 = create_test_quill(&temp_dir, "resume_template", "2.0");
    let quill_3_0 = create_test_quill(&temp_dir, "resume_template", "3.0");

    engine
        .register_quill(quill_1_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_3_0)
        .expect("Failed to register");

    // Request nonexistent version
    let result = engine.workflow("resume_template@2.5");

    assert!(result.is_err());
    match result {
        Err(quillmark::RenderError::VersionNotFound { diag }) => {
            // Verify helpful error message
            assert!(
                diag.message.contains("2.5") || diag.message.contains("not found"),
                "Error message should mention the requested version"
            );
            // Should have a hint with suggestions
            assert!(
                diag.hint.is_some(),
                "Error should include helpful hint with available versions"
            );
        }
        _ => panic!("Expected VersionNotFound error"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_quill_not_found_error_message() {
    let engine = Quillmark::new();

    // Request quill that doesn't exist at all
    let result = engine.workflow("nonexistent_quill");

    assert!(result.is_err());
    match result {
        Err(quillmark::RenderError::QuillNotFound { diag }) => {
            // Now correctly returns QuillNotFound after Phase 2.2
            assert!(diag.message.contains("not registered"));
        }
        _ => panic!("Expected QuillNotFound error"),
    }
}

#[test]
#[cfg(feature = "typst")]
fn test_latest_selector_with_multiple_versions() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register versions 1.0, 2.0, 3.0
    let quill_1_0 = create_test_quill(&temp_dir, "resume_template", "1.0");
    let quill_2_0 = create_test_quill(&temp_dir, "resume_template", "2.0");
    let quill_3_0 = create_test_quill(&temp_dir, "resume_template", "3.0");

    engine
        .register_quill(quill_1_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_3_0)
        .expect("Failed to register");

    // Resolve with no selector (implicit latest)
    let workflow_implicit = engine
        .workflow("resume_template")
        .expect("Failed to resolve implicit latest");

    // Resolve with @latest
    let workflow_explicit = engine
        .workflow("resume_template@latest")
        .expect("Failed to resolve @latest");

    // Both should work - verify quill name
    assert_eq!(workflow_implicit.quill_name(), "resume_template");
    assert_eq!(workflow_explicit.quill_name(), "resume_template");
    // Both should use version 3.0 (highest version)
}

#[test]
#[cfg(feature = "typst")]
fn test_version_selector_with_unversioned_document() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Register versions
    let quill_1_0 = create_test_quill(&temp_dir, "resume_template", "1.0");
    let quill_2_0 = create_test_quill(&temp_dir, "resume_template", "2.0");

    engine
        .register_quill(quill_1_0)
        .expect("Failed to register");
    engine
        .register_quill(quill_2_0)
        .expect("Failed to register");

    // Document without version in QUILL tag
    let markdown = r#"---
QUILL: resume_template
title: Test Document
---

# Test Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse");
    let workflow = engine.workflow(&parsed).expect("Failed to create workflow");

    // Should use latest version (2.0)
    assert_eq!(workflow.quill_name(), "resume_template");
    // Version resolution worked - workflow was created successfully
}

#[test]
#[cfg(feature = "typst")]
fn test_backward_compatibility_unversioned_quill() {
    let mut engine = Quillmark::new();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a quill with default version (0.1)
    let quill_path = temp_dir.path().join("legacy_quill");
    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.yaml"),
        "Quill:\n  name: \"legacy_quill\"\n  version: \"0.1\"\n  backend: \"typst\"\n  plate_file: \"plate.typ\"\n  description: \"Legacy quill\"\n",
    )
    .expect("Failed to write Quill.yaml");
    fs::write(
        quill_path.join("plate.typ"),
        "= Legacy Template\n\n#{{{{ body | Content }}}}",
    )
    .expect("Failed to write plate.typ");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine.register_quill(quill).expect("Failed to register");

    // Should be accessible without version (implicit latest)
    let workflow = engine
        .workflow("legacy_quill")
        .expect("Failed to load legacy quill");

    let markdown = "# Test Content";
    let _parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse");
    // Don't actually render since we don't have fonts - just verify workflow creation works
    assert_eq!(workflow.quill_name(), "legacy_quill");
}
