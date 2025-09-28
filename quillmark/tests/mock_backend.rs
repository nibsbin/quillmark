mod common;

use common::MockBackend;
use quillmark_core::{Backend, RenderConfig, OutputFormat};
use quillmark::render;
use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

// Helper to create a simple mock quill template
fn create_mock_quill_template(temp_dir: &TempDir) -> PathBuf {
    let quill_path = temp_dir.path().join("mock_quill");
    fs::create_dir_all(&quill_path).expect("Failed to create quill directory");
    
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
    
    let options = RenderConfig {
        backend: Box::new(MockBackend),
        output_format: Some(OutputFormat::Txt),
        quill_path: quill_path,
    };

    let result = render("# Hello World", &options);
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
    
    let options = RenderConfig {
        backend: Box::new(MockBackend),
        output_format: Some(OutputFormat::Svg),
        quill_path: quill_path,
    };

    let result = render("# Hello World", &options);
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
    
    let options = RenderConfig {
        backend: Box::new(MockBackend),
        output_format: Some(OutputFormat::Pdf),
        quill_path: quill_path,
    };

    let result = render("# Hello World", &options);
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
    
    let options = RenderConfig {
        backend: Box::new(MockBackend),
        output_format: None, // Should default to Txt
        quill_path: quill_path,
    };

    let result = render("# Hello World", &options);
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
    
    let options = RenderConfig {
        backend: Box::new(MockBackend),
        output_format: Some(OutputFormat::Txt),
        quill_path: quill_path,
    };

    let result = render("", &options);
    assert!(result.is_ok());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
    assert!(content.contains("empty"));
    assert!(content.contains("Mock text output"));
}
