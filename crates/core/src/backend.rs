//! Backend trait for output backends.

use crate::error::RenderError;
use crate::quill::Quill;
use crate::{LiveSession, OutputFormat};

#[doc(hidden)]
pub mod sealed {
    /// The seal on [`Backend`](super::Backend), implemented by the workspace's
    /// own backends. Naming it from elsewhere names a hidden item, which is the
    /// declaration that the seam behind it moves without notice.
    pub trait Sealed {}
}

/// Backend trait for rendering different output formats.
///
/// # Implementing this outside the workspace
///
/// Unsupported, and the trait's shape is why. [`Backend::open`] returns a
/// [`LiveSession`], which only a `SessionHandle` implementation can build,
/// through `LiveSession::new`, and both of those are `#[doc(hidden)]`, so an
/// out-of-workspace backend writes against items this crate neither documents
/// nor holds stable.
///
/// The [`sealed::Sealed`] supertrait states that in the type system. It is a
/// declaration, not a barrier: a crate willing to name a `#[doc(hidden)]`
/// module can implement both. What it buys is that adding a method here stays a
/// minor release, since no implementation outside the workspace is one this
/// crate promised to keep compiling.
pub trait Backend: sealed::Sealed + Send + Sync + std::fmt::Debug {
    /// Get the backend identifier (e.g., "typst", "latex").
    fn id(&self) -> &'static str;

    /// Get supported output formats.
    fn supported_formats(&self) -> &'static [OutputFormat];

    /// Open a live render session from a quill and compiled JSON data.
    ///
    /// The backend pulls whatever static inputs it needs straight from
    /// `source` ([`Quill::files`] for assets, [`Quill::config`] for
    /// backend-specific config). There is no universal "template" input: a
    /// template/plate is one backend's private notion, read by that backend
    /// from its own files, not a parameter every backend must accept.
    fn open(
        &self,
        source: &Quill,
        json_data: &serde_json::Value,
    ) -> Result<LiveSession, RenderError>;
}

/// The refusal every backend owes a format outside its
/// [`Backend::supported_formats`], under `backend::format_not_supported`:
/// the one code a caller matches for this condition, so it is built once here
/// rather than once per backend.
///
/// `backend` names the backend in the message; `supported` becomes the hint.
pub fn unsupported_format(format: OutputFormat, backend: &str, supported: &[OutputFormat]) -> RenderError {
    RenderError::from_diag(
        crate::Diagnostic::new(
            crate::Severity::Error,
            format!("{format:?} not supported by the {backend} backend"),
        )
        .with_code("backend::format_not_supported".to_string())
        .with_hint(format!("Supported formats: {supported:?}")),
    )
}

/// Pre-session hint for whether a backend with these `formats` can paint pages
/// to a canvas, used before a session exists (e.g. a GUI deciding whether to
/// mount a canvas preview without first paying to open one).
///
/// Canvas paint needs a per-page *visual image* of the laid-out page, so the
/// predicate keys off the visual-page output formats ([`OutputFormat::Png`]
/// (raster) and [`OutputFormat::Svg`] (vector)) as opposed to
/// [`OutputFormat::Pdf`] (a document). A backend that can rasterize a page
/// advertises one of these in [`Backend::supported_formats`].
///
/// This is only a hint. The **authoritative** answer is
/// [`LiveSession::supports_canvas`](crate::LiveSession::supports_canvas),
/// which is derived from the session's actual canvas seam
/// ([`SessionHandle::page_size_pt`](crate::session::SessionHandle::page_size_pt));
/// there is no separately maintained capability flag to drift from the
/// implementation (a canvas backend pairs `render_rgba` with `page_size_pt`).
pub fn formats_support_canvas(formats: &[OutputFormat]) -> bool {
    formats
        .iter()
        .any(|f| matches!(f, OutputFormat::Png | OutputFormat::Svg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_support_canvas_keys_off_visual_formats() {
        use OutputFormat::*;
        // A visual-page format present → canvas (Typst and pdfform both emit Svg/Png).
        assert!(formats_support_canvas(&[Pdf, Svg, Png]));
        assert!(formats_support_canvas(&[Pdf, Svg]));
        assert!(formats_support_canvas(&[Png]));
        // No visual-page format → no canvas.
        assert!(!formats_support_canvas(&[Pdf]));
        assert!(!formats_support_canvas(&[]));
    }
}
