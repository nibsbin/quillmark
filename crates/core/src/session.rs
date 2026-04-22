use crate::{Diagnostic, RenderError, RenderOptions, RenderResult};

#[doc(hidden)]
pub trait SessionHandle: Send + Sync {
    fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError>;
    fn page_count(&self) -> usize;
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

    /// Attach a non-fatal warning to this session. The warning is appended to
    /// [`RenderResult::warnings`] on each call to [`RenderSession::render`].
    pub fn with_warning(mut self, warning: Option<Diagnostic>) -> Self {
        self.warning = warning;
        self
    }

    pub fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    pub fn render(&self, opts: &RenderOptions) -> Result<RenderResult, RenderError> {
        let mut result = self.inner.render(opts)?;
        if let Some(warning) = &self.warning {
            result.warnings.push(warning.clone_without_source());
        }
        Ok(result)
    }
}
