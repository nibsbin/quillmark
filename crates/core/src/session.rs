use std::any::Any;

use crate::{Diagnostic, RenderError, RenderOptions, RenderResult};

/// Backend-specific session implementation.
///
/// Implementors must be `'static` (required by `Any`), `Send`, and `Sync`. The
/// `'static` bound prevents borrowing source data — own anything you need to
/// keep alive for the session's lifetime.
#[doc(hidden)]
pub trait SessionHandle: Any + Send + Sync {
    fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError>;
    fn page_count(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
}

/// Opaque, backend-backed iterative render session.
pub struct RenderSession {
    inner: Box<dyn SessionHandle>,
    warnings: Vec<Diagnostic>,
}

impl RenderSession {
    #[doc(hidden)]
    pub fn new(inner: Box<dyn SessionHandle>) -> Self {
        Self {
            inner,
            warnings: Vec::new(),
        }
    }

    /// Borrow the underlying [`SessionHandle`] for typed-side-channel access.
    ///
    /// Bindings call this and downcast via [`SessionHandle::as_any`] to reach
    /// backend-specific surfaces (e.g. `quillmark_typst::typst_session_of`
    /// for canvas preview). Intentionally `#[doc(hidden)]` — the shape of
    /// this accessor is not part of the stable public API.
    #[doc(hidden)]
    pub fn handle(&self) -> &dyn SessionHandle {
        &*self.inner
    }

    /// Attach session-level warnings. Appended to [`RenderResult::warnings`]
    /// on every [`RenderSession::render`] call and surfaced verbatim by
    /// [`RenderSession::warnings`].
    pub fn with_warnings(mut self, warnings: Vec<Diagnostic>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    /// Snapshot of session-level warnings attached at `Backend::open` time.
    ///
    /// Empty when the backend produced none. These are also appended to
    /// [`RenderResult::warnings`] on each [`RenderSession::render`] call;
    /// this accessor surfaces them to consumers (e.g. canvas previews) that
    /// don't go through `render()`.
    pub fn warnings(&self) -> &[Diagnostic] {
        &self.warnings
    }

    pub fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
        let mut result = self.inner.render(opts)?;
        result.warnings.extend(self.warnings.iter().cloned());
        Ok(result)
    }
}
