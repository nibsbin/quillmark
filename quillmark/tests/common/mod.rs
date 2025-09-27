use quillmark_core::{Artifact, Backend, Options, OutputFormat, RenderError};

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
