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
}
