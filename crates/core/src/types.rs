//! Core types for rendering and output formats.

/// Output formats supported by backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub enum OutputFormat {
    /// Scalable Vector Graphics output
    Svg,
    /// Portable Document Format output
    Pdf,
    /// Portable Network Graphics output (raster)
    Png,
}

impl OutputFormat {
    /// Every output format, in a stable order. Bindings enumerate this for
    /// choice lists and error messages instead of hand-listing the variants.
    ///
    /// A slice, not an array: an array's length is part of its type, so a
    /// fourth format would break every caller that names this constant's type
    /// even though [`OutputFormat`] itself is `#[non_exhaustive]`.
    pub const ALL: &'static [OutputFormat] =
        &[OutputFormat::Pdf, OutputFormat::Svg, OutputFormat::Png];

    /// The lowercase string id (`"pdf"`, `"svg"`, `"png"`). This is the
    /// single source of truth for the format ↔ string mapping every binding and
    /// the CLI share.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Pdf => "pdf",
            OutputFormat::Svg => "svg",
            OutputFormat::Png => "png",
        }
    }

    /// The IANA MIME type for this format. Single source of truth for the
    /// format ↔ MIME mapping shared across bindings.
    pub fn mime_type(&self) -> &'static str {
        match self {
            OutputFormat::Pdf => "application/pdf",
            OutputFormat::Svg => "image/svg+xml",
            OutputFormat::Png => "image/png",
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a string does not name an [`OutputFormat`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutputFormatError(pub String);

impl std::fmt::Display for ParseOutputFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let choices: Vec<&str> = OutputFormat::ALL.iter().map(|fmt| fmt.as_str()).collect();
        write!(
            f,
            "Invalid output format: {}. Must be one of: {}",
            self.0,
            choices.join(", ")
        )
    }
}

impl std::error::Error for ParseOutputFormatError {}

impl std::str::FromStr for OutputFormat {
    type Err = ParseOutputFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pdf" => Ok(OutputFormat::Pdf),
            "svg" => Ok(OutputFormat::Svg),
            "png" => Ok(OutputFormat::Png),
            other => Err(ParseOutputFormatError(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn output_format_str_round_trips() {
        for &fmt in OutputFormat::ALL {
            assert_eq!(OutputFormat::from_str(fmt.as_str()), Ok(fmt));
            // Display matches as_str, and parsing is case-insensitive.
            assert_eq!(fmt.to_string(), fmt.as_str());
            assert_eq!(
                OutputFormat::from_str(&fmt.as_str().to_uppercase()),
                Ok(fmt)
            );
        }
    }

    #[test]
    fn output_format_parse_error_lists_choices() {
        let err = OutputFormat::from_str("docx").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("docx"), "{msg}");
        for &fmt in OutputFormat::ALL {
            assert!(
                msg.contains(fmt.as_str()),
                "choices missing {}: {msg}",
                fmt.as_str()
            );
        }
    }
}

/// An artifact produced by rendering.
#[derive(Debug)]
#[non_exhaustive]
pub struct Artifact {
    /// The binary content of the artifact
    pub bytes: Vec<u8>,
    /// The format of the output
    pub output_format: OutputFormat,
}

impl Artifact {
    /// The two facts an artifact always carries.
    pub fn new(bytes: Vec<u8>, output_format: OutputFormat) -> Self {
        Self {
            bytes,
            output_format,
        }
    }
}

/// Internal rendering options.
///
/// Built from [`Default`] and narrowed by the `with_*` setters:
///
/// ```
/// use quillmark_core::{OutputFormat, RenderOptions};
///
/// let opts = RenderOptions::default()
///     .with_output_format(OutputFormat::Png)
///     .with_ppi(300.0);
/// ```
///
/// `#[non_exhaustive]` forbids every out-of-crate struct expression, functional
/// update included, so the setters are the construction path rather than a
/// convenience beside one. Reading and assigning an individual field still
/// works — `opts.ppi = Some(300.0)` is equivalent to the setter.
///
/// This is the render surface's growth point: a new option is a new field here.
/// Under the attribute that costs a caller nothing.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RenderOptions {
    /// Optional output format specification
    pub output_format: Option<OutputFormat>,
    /// Pixels per inch for raster output formats (e.g., PNG).
    /// Ignored for vector/document formats (PDF, SVG).
    /// Defaults to 144.0 (2x at 72pt/inch) when `None`.
    pub ppi: Option<f32>,
    /// Optional 0-based page indices to render (e.g., `vec![0, 2]` for
    /// the first and third pages). `None` renders all pages. Any index
    /// `>= page_count` fails with a `RenderError` carrying the
    /// `typst::page_index_out_of_bounds` code — call
    /// `LiveSession::page_count()` first if validation is needed.
    /// Backends that do not support page selection (notably PDF) fail with a
    /// `RenderError` carrying `typst::pdf_page_selection_not_supported` when
    /// this is `Some`.
    pub pages: Option<Vec<usize>>,
    /// Override for the PDF `/Info` `/Producer` metadata string. `None` uses
    /// the backend default (`Quillmark <version>` for the Typst backend).
    /// Applies to PDF output only; ignored by SVG/PNG.
    pub producer: Option<String>,
    /// Populate [`RenderResult::regions`](crate::RenderResult) with the
    /// schema-field geometry sidecar (the same entries
    /// [`LiveSession::regions`](crate::LiveSession::regions) serves), for
    /// consumers without a live session — static overlays over exported SVG,
    /// PDF post-processing, CI coverage probes. Default `false`: exports pay
    /// no introspection cost, and the sidecar stays a request, not a promise.
    ///
    /// The sidecar always describes the **whole document** — page indices are
    /// document-space and unaffected by a `pages` subset selection — so it
    /// never disagrees with the geometry of the compile it came from.
    pub regions: bool,
}

impl RenderOptions {
    /// Set [`output_format`](Self::output_format).
    pub fn with_output_format(mut self, output_format: OutputFormat) -> Self {
        self.output_format = Some(output_format);
        self
    }

    /// Set [`ppi`](Self::ppi).
    pub fn with_ppi(mut self, ppi: f32) -> Self {
        self.ppi = Some(ppi);
        self
    }

    /// Set [`pages`](Self::pages).
    pub fn with_pages(mut self, pages: Vec<usize>) -> Self {
        self.pages = Some(pages);
        self
    }

    /// Set [`producer`](Self::producer).
    pub fn with_producer(mut self, producer: String) -> Self {
        self.producer = Some(producer);
        self
    }

    /// Request the geometry sidecar ([`regions`](Self::regions)).
    pub fn with_regions(mut self, regions: bool) -> Self {
        self.regions = regions;
        self
    }
}
