mod common;

use common::MockBackend;
use quillmark::{QuillEngine, OutputFormat};
use quillmark_core::Backend; // Import Backend trait so we can use trait methods
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

// Helper to create a simple mock quill template
fn create_mock_quill_template(temp_dir: &TempDir) -> PathBuf {
    let quill_path = temp_dir.path().join("mock_quill");
    fs::create_dir_all(&quill_path).expect("Failed to create quill directory");
    
    // Create quill.toml to specify the correct glue file
    let toml_content = r#"[Quill]
name = "mock-quill"
glue_file = "glue.txt"
"#;
    fs::write(quill_path.join("quill.toml"), toml_content).expect("Failed to write quill.toml");
    
    let template_content = "{{ body }}";
    let template_file = quill_path.join("glue.txt");
    fs::write(&template_file, template_content).expect("Failed to write template file");
    
    quill_path
}

#[test]
fn test_mock_backend_id() {
    let backend = MockBackend;
    assert_eq!(backend.id(), "mock");
}

#[test]
fn test_mock_backend_supported_formats() {
    let backend = MockBackend;
    let formats = backend.supported_formats();
    assert!(formats.contains(&OutputFormat::Txt));
    assert!(formats.contains(&OutputFormat::Svg));
    assert!(formats.contains(&OutputFormat::Pdf));
    assert_eq!(formats.len(), 3);
}

#[test]
fn test_mock_backend_glue_type() {
    let backend = MockBackend;
    assert_eq!(backend.glue_type(), ".txt");
}

#[test]
fn test_mock_backend_render_txt() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let quill_path = create_mock_quill_template(&temp_dir);
    
    let engine = QuillEngine::new(Box::new(MockBackend), quill_path).expect("Failed to create engine");

    let result = engine.render_with_format("# Hello World", Some(OutputFormat::Txt));
    assert!(result.is_ok());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].output_format, OutputFormat::Txt);

    let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
    assert!(content.contains("Hello World"));
    assert!(content.contains("Mock text output"));
}

#[test]
fn test_mock_backend_render_svg() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let quill_path = create_mock_quill_template(&temp_dir);
    
    let engine = QuillEngine::new(Box::new(MockBackend), quill_path).expect("Failed to create engine");

    let result = engine.render_with_format("# Hello World", Some(OutputFormat::Svg));
    assert!(result.is_ok());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].output_format, OutputFormat::Svg);

    let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
    assert!(content.contains("Hello World"));
    assert!(content.contains("Mock SVG output"));
    assert!(content.contains("<svg>"));
}

#[test]
fn test_mock_backend_render_pdf() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let quill_path = create_mock_quill_template(&temp_dir);
    
    let engine = QuillEngine::new(Box::new(MockBackend), quill_path).expect("Failed to create engine");

    let result = engine.render_with_format("# Hello World", Some(OutputFormat::Pdf));
    assert!(result.is_ok());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].output_format, OutputFormat::Pdf);

    let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
    assert!(content.contains("Hello World"));
    assert!(content.contains("Mock PDF output"));
}

#[test]
fn test_mock_backend_render_default_format() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let quill_path = create_mock_quill_template(&temp_dir);
    
    let engine = QuillEngine::new(Box::new(MockBackend), quill_path).expect("Failed to create engine");

    let result = engine.render("# Hello World"); // No format specified, should default to Txt
    assert!(result.is_ok());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].output_format, OutputFormat::Txt);

    let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
    assert!(content.contains("Hello World"));
    assert!(content.contains("Mock text output"));
}

#[test]
fn test_mock_backend_render_empty_markdown() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let quill_path = create_mock_quill_template(&temp_dir);
    
    let engine = QuillEngine::new(Box::new(MockBackend), quill_path).expect("Failed to create engine");

    let result = engine.render_with_format("", Some(OutputFormat::Txt));
    assert!(result.is_ok());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
    assert!(content.contains("empty"));
    assert!(content.contains("Mock text output"));
}

#[test]
fn test_quill_engine_properties() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let quill_path = create_mock_quill_template(&temp_dir);
    
    let engine = QuillEngine::new(Box::new(MockBackend), quill_path).expect("Failed to create engine");

    // Test engine properties
    assert_eq!(engine.backend_id(), "mock");
    assert_eq!(engine.quill_name(), "mock-quill");
    assert_eq!(engine.glue_type(), ".txt");
    assert_eq!(engine.supported_formats(), &[OutputFormat::Txt, OutputFormat::Svg, OutputFormat::Pdf]);
}
