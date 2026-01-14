//! # Backend Registration Tests
//!
//! Tests for custom backend registration and management.
//!
//! ## Test Coverage
//!
//! This test suite validates:
//! - **Basic registration** - Registering custom backends
//! - **Multiple backends** - Multiple backend registration
//! - **Backend replacement** - Re-registering with same ID
//! - **Workflow creation** - Using custom backends in workflows
//! - **Integration** - End-to-end rendering with custom backends
//!
//! ## Mock Backend
//!
//! Uses a minimal `MockBackend` implementation that:
//! - Implements the full Backend trait
//! - Supports TXT output format
//! - Returns plate content as-is (for testing)
//! - Provides minimal filter registration
//!
//! ## Purpose
//!
//! Validates that the backend extension system works correctly and that
//! third-party backends can integrate cleanly with the engine.

use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark, RenderError};
use quillmark_core::{Artifact, Backend, RenderOptions, RenderResult};
use std::fs;
use tempfile::TempDir;

/// A mock backend for testing purposes
struct MockBackend {
    id: &'static str,
}

impl Backend for MockBackend {
    fn id(&self) -> &'static str {
        self.id
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        &[OutputFormat::Txt]
    }

    fn plate_extension_types(&self) -> &'static [&'static str] {
        &[".txt"]
    }

    fn allow_auto_plate(&self) -> bool {
        true
    }

    fn compile(
        &self,
        plated: &str,
        _quill: &Quill,
        _opts: &RenderOptions,
    ) -> Result<RenderResult, RenderError> {
        // Simple mock: just return the plated content as a text artifact
        let artifacts = vec![Artifact {
            bytes: plated.as_bytes().to_vec(),
            output_format: OutputFormat::Txt,
        }];
        Ok(RenderResult::new(artifacts, OutputFormat::Txt))
    }
}

#[test]
fn test_register_backend_basic() {
    let mut engine = Quillmark::new();

    // Create a mock backend
    let mock_backend = Box::new(MockBackend { id: "mock" });

    // Register the backend
    engine.register_backend(mock_backend);

    // Check that backend is registered
    let backends = engine.registered_backends();
    assert!(backends.contains(&"mock"));
}

#[test]
fn test_register_multiple_backends() {
    let mut engine = Quillmark::new();

    // Register two different mock backends
    engine.register_backend(Box::new(MockBackend { id: "mock1" }));
    engine.register_backend(Box::new(MockBackend { id: "mock2" }));

    // Check that both backends are registered
    let backends = engine.registered_backends();
    assert!(backends.contains(&"mock1"));
    assert!(backends.contains(&"mock2"));
}

#[test]
fn test_register_backend_replaces_existing() {
    let mut engine = Quillmark::new();

    // Register a backend with ID "custom"
    engine.register_backend(Box::new(MockBackend { id: "custom" }));

    let backends = engine.registered_backends();
    assert!(backends.contains(&"custom"));

    // Register another backend with the same ID
    engine.register_backend(Box::new(MockBackend { id: "custom" }));

    // Should still only have one "custom" backend (replaced)
    let backends = engine.registered_backends();
    assert_eq!(backends.iter().filter(|&&b| b == "custom").count(), 1);
}

#[test]
fn test_workflow_with_custom_backend() {
    let mut engine = Quillmark::new();

    // Register a custom backend
    engine.register_backend(Box::new(MockBackend { id: "mock-txt" }));

    // Create a test quill that uses our custom backend
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let quill_path = temp_dir.path().join("test-quill");

    fs::create_dir_all(&quill_path).expect("Failed to create quill dir");
    fs::write(
        quill_path.join("Quill.toml"),
        "[Quill]\nname = \"custom-backend-quill\"\nbackend = \"mock-txt\"\nplate_file = \"plate.txt\"\ndescription = \"Test quill with custom backend\"\n",
    )
    .expect("Failed to write Quill.toml");
    fs::write(quill_path.join("plate.txt"), "Test template: {{ title }}")
        .expect("Failed to write plate.txt");

    let quill = Quill::from_path(quill_path).expect("Failed to load quill");
    engine
        .register_quill(quill)
        .expect("Failed to register quill");

    // Load workflow using the custom backend
    let workflow = engine
        .workflow("custom-backend-quill")
        .expect("Failed to load workflow");

    // Verify workflow properties
    assert_eq!(workflow.backend_id(), "mock-txt");
    assert_eq!(workflow.quill_name(), "custom-backend-quill");
    assert!(workflow.supported_formats().contains(&OutputFormat::Txt));

    // Test rendering with the custom backend
    let markdown = r#"---
title: Hello Custom Backend
---

# Test Content
"#;

    let parsed = ParsedDocument::from_markdown(markdown).expect("Failed to parse markdown");
    let result = workflow
        .render(&parsed, Some(OutputFormat::Txt))
        .expect("Failed to render");

    assert!(!result.artifacts.is_empty());
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Txt);
}

#[test]
fn test_register_backend_after_new() {
    // Test that we can add backends after creating the engine
    let mut engine = Quillmark::new();

    let initial_count = engine.registered_backends().len();

    engine.register_backend(Box::new(MockBackend { id: "added-later" }));

    let backends = engine.registered_backends();
    assert_eq!(backends.len(), initial_count + 1);
    assert!(backends.contains(&"added-later"));
}
