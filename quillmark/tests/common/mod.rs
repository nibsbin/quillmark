use quillmark_core::{Artifact, Backend, RenderConfig, OutputFormat, RenderError, QuillData, Glue};

// use minijinja-compatible filter API types
use quillmark_core::templating::filter_api::{State, Value as MjValue, Kwargs, Error as MjError};

/// Mock backend for testing purposes
pub struct MockBackend;

/// Mock filter that just echoes the input (MiniJinja-compatible)
fn mock_filter(_state: &State, value: MjValue, _kwargs: Kwargs) -> Result<MjValue, MjError> {
    Ok(value)
}

impl Backend for MockBackend {
    fn id(&self) -> &'static str {
        "mock"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        // Mock backend supports all formats
        &[OutputFormat::Txt, OutputFormat::Svg, OutputFormat::Pdf]
    }

    fn glue_type(&self) -> &'static str {
        ".txt"
    }

    fn register_filters(&self, glue: &mut Glue) {
        // register the minijinja-style filter function
        glue.register_filter("mock", mock_filter);
    }

    fn compile(&self, glue_content: &str, _quill_data: &QuillData, opts: &RenderConfig) -> Result<Vec<Artifact>, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Txt);

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
                glue_content.lines().next().unwrap_or("empty")
            ),
            OutputFormat::Svg => format!(
                "<svg><text>Mock SVG output for: {}</text></svg>",
                glue_content.lines().next().unwrap_or("empty")
            ),
            OutputFormat::Pdf => format!(
                "Mock PDF output for: {}",
                glue_content.lines().next().unwrap_or("empty")
            ),
        };

        Ok(vec![Artifact {
            bytes: mock_content.into_bytes(),
            output_format: format,
        }])
    }
}

