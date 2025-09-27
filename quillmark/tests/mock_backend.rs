mod common;

use common::MockBackend;
use quillmark_core::{Backend, Options, OutputFormat};

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
fn test_mock_backend_render_txt() {
    let backend = MockBackend;
    let options = Options {
        backend: Some("mock".to_string()),
        format: Some(OutputFormat::Txt),
    };

    let result = backend.render("# Hello World", &options);
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
    let backend = MockBackend;
    let options = Options {
        backend: Some("mock".to_string()),
        format: Some(OutputFormat::Svg),
    };

    let result = backend.render("# Hello World", &options);
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
    let backend = MockBackend;
    let options = Options {
        backend: Some("mock".to_string()),
        format: Some(OutputFormat::Pdf),
    };

    let result = backend.render("# Hello World", &options);
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
    let backend = MockBackend;
    let options = Options {
        backend: Some("mock".to_string()),
        format: None, // Should default to Txt
    };

    let result = backend.render("# Hello World", &options);
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
    let backend = MockBackend;
    let options = Options {
        backend: Some("mock".to_string()),
        format: Some(OutputFormat::Txt),
    };

    let result = backend.render("", &options);
    assert!(result.is_ok());

    let artifacts = result.unwrap();
    assert_eq!(artifacts.len(), 1);

    let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
    assert!(content.contains("empty"));
    assert!(content.contains("Mock text output"));
}
