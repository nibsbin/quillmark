#![doc = include_str!("lib.md")]

pub mod compile;
pub mod convert;
mod filters;
mod world;
use filters::{
    asset_filter, content_filter, date_filter, dict_filter, lines_filter, string_filter,
};
use quillmark_core::{Artifact, Backend, Glue, OutputFormat, Quill, RenderError, RenderOptions};

/// Typst backend implementation for Quillmark.
pub struct TypstBackend;

impl Backend for TypstBackend {
    fn id(&self) -> &'static str {
        "typst"
    }

    fn supported_formats(&self) -> &'static [OutputFormat] {
        &[OutputFormat::Pdf, OutputFormat::Svg]
    }

    fn glue_type(&self) -> &'static str {
        ".typ"
    }

    fn register_filters(&self, glue: &mut Glue) {
        // Register basic filters (simplified for now)
        glue.register_filter("String", string_filter);
        glue.register_filter("Lines", lines_filter);
        glue.register_filter("Date", date_filter);
        glue.register_filter("Dict", dict_filter);
        glue.register_filter("Content", content_filter);
        glue.register_filter("Asset", asset_filter);
    }

    fn compile(
        &self,
        glued_content: &str,
        quill: &Quill,
        opts: &RenderOptions,
    ) -> Result<Vec<Artifact>, RenderError> {
        let format = opts.output_format.unwrap_or(OutputFormat::Pdf);

        // Check if format is supported
        if !self.supported_formats().contains(&format) {
            return Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format,
            });
        }

        println!("Typst backend compiling for quill: {}", quill.name);

        match format {
            OutputFormat::Pdf => {
                let bytes = compile::compile_to_pdf(quill, glued_content).unwrap();
                Ok(vec![Artifact {
                    bytes: bytes,
                    output_format: OutputFormat::Pdf,
                }])
            }
            OutputFormat::Svg => {
                let svg_pages = compile::compile_to_svg(quill, glued_content).map_err(|e| {
                    RenderError::Other(format!("SVG compilation failed: {}", e).into())
                })?;
                Ok(svg_pages
                    .into_iter()
                    .map(|bytes| Artifact {
                        bytes,
                        output_format: OutputFormat::Svg,
                    })
                    .collect())
            }
            OutputFormat::Txt => Err(RenderError::FormatNotSupported {
                backend: self.id().to_string(),
                format: OutputFormat::Txt,
            }),
        }
    }
}

impl Default for TypstBackend {
    /// Creates a new [`TypstBackend`] instance.
    fn default() -> Self {
        Self
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_info() {
        let backend = TypstBackend::default();
        assert_eq!(backend.id(), "typst");
        assert_eq!(backend.glue_type(), ".typ");
        assert!(backend.supported_formats().contains(&OutputFormat::Pdf));
        assert!(backend.supported_formats().contains(&OutputFormat::Svg));
    }
}

// Re-export for compatibility
pub use TypstBackend as backend;
