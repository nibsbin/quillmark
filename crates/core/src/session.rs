use std::any::Any;

use crate::{Diagnostic, RenderError, RenderOptions, RenderResult};

#[doc(hidden)]
pub trait SessionHandle: Any + Send + Sync {
    fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError>;
    fn page_count(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
}

/// Opaque, backend-backed iterative render session.
pub struct RenderSession {
    inner: Box<dyn SessionHandle>,
    warning: Option<Diagnostic>,
}

impl RenderSession {
    #[doc(hidden)]
    pub fn new(inner: Box<dyn SessionHandle>) -> Self {
        Self {
            inner,
            warning: None,
        }
    }

    #[doc(hidden)]
    pub fn handle(&self) -> &dyn SessionHandle {
        &*self.inner
    }

    /// Attach a non-fatal warning to this session. The warning is appended to
    /// [`RenderResult::warnings`] on each call to [`RenderSession::render`].
    pub fn with_warning(mut self, warning: Option<Diagnostic>) -> Self {
        self.warning = warning;
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
    pub fn warnings(&self) -> Vec<Diagnostic> {
        self.warning.iter().cloned().collect()
    }

    pub fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
        let mut result = self.inner.render(opts)?;
        if let Some(warning) = &self.warning {
            result.warnings.push(warning.clone());
        }
        Ok(result)
    }
}
