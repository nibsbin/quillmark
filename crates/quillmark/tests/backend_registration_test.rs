//! # Backend Registration Tests

use quillmark::{OutputFormat, ParsedDocument, Quill, Quillmark, RenderError};
use quillmark_core::{Artifact, Backend, RenderOptions, RenderResult};
use std::fs;
use tempfile::TempDir;

#[derive(Debug)]
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

    fn compile(
        &self,
        plated: &str,
        _quill: &Quill,
        _opts: &RenderOptions,
        _json_data: &serde_json::Value,
    ) -> Result<RenderResult, RenderError> {
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
    engine.register_backend(Box::new(MockBackend { id: "mock" }));
    let backends = engine.registered_backends();
    assert!(backends.contains(&"mock"));
}

#[test]
fn test_register_multiple_backends() {
    let mut engine = Quillmark::new();
    engine.register_backend(Box::new(MockBackend { id: "mock1" }));
    engine.register_backend(Box::new(MockBackend { id: "mock2" }));
    let backends = engine.registered_backends();
    assert!(backends.contains(&"mock1"));
    assert!(backends.contains(&"mock2"));
}

#[test]
fn test_register_backend_replaces_existing() {
    let mut engine = Quillmark::new();
    engine.register_backend(Box::new(MockBackend { id: "custom" }));
    engine.register_backend(Box::new(MockBackend { id: "custom" }));
    let backends = engine.registered_backends();
    assert_eq!(backends.iter().filter(|&&b| b == "custom").count(), 1);
}

#[test]
fn test_workflow_with_custom_backend() {
    let mut engine = Quillmark::new();
    engine.register_backend(Box::new(MockBackend { id: "mock-txt" }));

    let temp_dir = TempDir::new().unwrap();
    let quill_path = temp_dir.path().join("test_quill");
    fs::create_dir_all(&quill_path).unwrap();
    fs::write(
        quill_path.join("Quill.yaml"),
        "Quill:\n  name: \"custom_backend_quill\"\n  version: \"1.0\"\n  backend: \"mock-txt\"\n  plate_file: \"plate.txt\"\n  description: \"Test\"\n",
    ).unwrap();
    fs::write(quill_path.join("plate.txt"), "Test template: {{ title }}").unwrap();

    let quill = engine
        .quill_from_path(&quill_path)
        .expect("quill_from_path failed");
    let workflow = engine.workflow(&quill).expect("workflow failed");

    assert_eq!(workflow.backend_id(), "mock-txt");
    assert!(workflow.quill_ref().starts_with("custom_backend_quill@"));
    assert!(workflow.supported_formats().contains(&OutputFormat::Txt));

    let markdown = "---\nQUILL: custom_backend_quill\ntitle: Hello Custom Backend\n---\n\n# Test\n";
    let parsed = ParsedDocument::from_markdown(markdown).expect("parse failed");
    let result = workflow
        .render(&parsed, Some(OutputFormat::Txt))
        .expect("render failed");

    assert!(!result.artifacts.is_empty());
    assert_eq!(result.artifacts[0].output_format, OutputFormat::Txt);
}

#[test]
fn test_register_backend_after_new() {
    let mut engine = Quillmark::new();
    let initial_count = engine.registered_backends().len();
    engine.register_backend(Box::new(MockBackend { id: "added-later" }));
    let backends = engine.registered_backends();
    assert_eq!(backends.len(), initial_count + 1);
    assert!(backends.contains(&"added-later"));
}

#[test]
fn test_register_backend_emits_no_warnings() {
    let mut engine = Quillmark::new();
    engine.register_backend(Box::new(MockBackend { id: "no-warning" }));
    assert!(engine.warnings().is_empty());
    assert!(engine.take_warnings().is_empty());
}
