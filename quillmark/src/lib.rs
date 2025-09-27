// Re-export all types from quillmark-core for backward compatibility
pub use quillmark_core::{Artifact, Backend, Options, OutputFormat, RenderError, RenderResult};

/// Render markdown using the specified options
///
/// This function will select an appropriate backend based on the options provided.
/// If no backend is specified, it will try to find a suitable one based on the
/// requested output format.
pub fn render(_markdown: &str, _options: &Options) -> RenderResult {
    // For now, return an error indicating no backends are available
    // This will be implemented when backends are registered
    Err(RenderError::UnsupportedBackend(
        "no backends available".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock backend for testing purposes
    pub struct MockBackend;

    impl Backend for MockBackend {
        fn id(&self) -> &'static str {
            "mock"
        }

        fn supported_formats(&self) -> &'static [OutputFormat] {
            // Mock backend supports all formats
            &[OutputFormat::Txt, OutputFormat::Svg, OutputFormat::Pdf]
        }

        fn render(&self, markdown: &str, opts: &Options) -> Result<Vec<Artifact>, RenderError> {
            let format = opts.format.unwrap_or(OutputFormat::Txt);

            // Check if the requested format is supported
            if !self.supported_formats().contains(&format) {
                return Err(RenderError::FormatNotSupported {
                    backend: self.id().to_string(),
                    format,
                });
            }

            let mock_content = match format {
                OutputFormat::Txt => format!(
                    "Mock text output for: {}",
                    markdown.lines().next().unwrap_or("empty")
                ),
                OutputFormat::Svg => format!(
                    "<svg><text>Mock SVG output for: {}</text></svg>",
                    markdown.lines().next().unwrap_or("empty")
                ),
                OutputFormat::Pdf => format!(
                    "Mock PDF output for: {}",
                    markdown.lines().next().unwrap_or("empty")
                ),
            };

            Ok(vec![Artifact {
                bytes: mock_content.into_bytes(),
                output_format: format,
            }])
        }
    }

    #[test]
    fn test_render_with_no_backends() {
        let options = Options {
            backend: None,
            format: Some(OutputFormat::Pdf),
        };

        let result = render("# Hello World", &options);
        assert!(result.is_err());

        match result.unwrap_err() {
            RenderError::UnsupportedBackend(_) => {}
            _ => panic!("Expected UnsupportedBackend error"),
        }
    }

    #[test]
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Pdf, OutputFormat::Pdf);
        assert_ne!(OutputFormat::Pdf, OutputFormat::Svg);
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
}
