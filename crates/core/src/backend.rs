//! Backend trait for output backends.

use crate::error::RenderError;
use crate::{OutputFormat, Quill, RenderSession};

/// Backend trait for rendering different output formats.
pub trait Backend: Send + Sync + std::fmt::Debug {
    /// Get the backend identifier (e.g., "typst", "latex").
    fn id(&self) -> &'static str;

    /// Get supported output formats.
    fn supported_formats(&self) -> &'static [OutputFormat];

    /// Open an iterative render session from plate + compiled JSON data.
    fn open(
        &self,
        plate_content: &str,
        quill: &Quill,
        json_data: &serde_json::Value,
    ) -> Result<RenderSession, RenderError>;
}
