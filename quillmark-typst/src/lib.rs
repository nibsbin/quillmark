use quillmark_core::{Backend, OutputFormat, Options, RenderError, Artifact};

/// Typst backend implementation using puldown-cmark and Typst
pub struct TypstBackend;

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        // Typst can output PDF and SVG
        &[OutputFormat::Pdf, OutputFormat::Svg]
    }

    fn render(&self, markdown: &str, _opts: &Options) -> Result<Vec<Artifact>, RenderError> {
        // This is a skeleton implementation
        // In a real implementation, this would:
        // 1. Use pulldown-cmark to convert markdown to Typst format
        // 2. Use Typst to compile to the requested output format
        // 3. Return the compiled artifacts
        
        // For now, return a mock artifact
        let mock_content = format!("Mock Typst output for: {}", 
            markdown.lines().next().unwrap_or("empty"));
        
        Ok(vec![Artifact {
            bytes: mock_content.into_bytes(),
            output_format: OutputFormat::Pdf,
        }])
    }
}

impl Default for TypstBackend {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quillmark_core::OutputFormat;

    #[test]
    fn test_typst_backend_id() {
        let backend = TypstBackend::default();
        assert_eq!(backend.id(), "typst");
    }

    #[test]
    fn test_typst_backend_supported_formats() {
        let backend = TypstBackend::default();
        let formats = backend.supported_formats();
        assert!(formats.contains(&OutputFormat::Pdf));
        assert!(formats.contains(&OutputFormat::Svg));
        assert!(!formats.contains(&OutputFormat::Txt));
    }

    #[test]
    fn test_typst_backend_render() {
        let backend = TypstBackend::default();
        let options = Options {
            backend: Some("typst".to_string()),
            format: Some(OutputFormat::Pdf),
        };
        
        let result = backend.render("# Hello World", &options);
        assert!(result.is_ok());
        
        let artifacts = result.unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].output_format, OutputFormat::Pdf);
        
        let content = String::from_utf8(artifacts[0].bytes.clone()).unwrap();
        assert!(content.contains("Hello World"));
    }
}