use quillmark_core::{Backend, Diagnostic, Quill, RenderError, Severity};
use std::collections::HashMap;
use std::sync::Arc;

use super::workflow::Workflow;

/// High-level engine for orchestrating backends and quills.
pub struct Quillmark {
    backends: HashMap<String, Arc<dyn Backend>>,
    warnings: Vec<Diagnostic>,
}

impl Quillmark {
    /// Create a new Quillmark with auto-registered backends based on enabled features.
    pub fn new() -> Self {
        let mut engine = Self {
            backends: HashMap::new(),
            warnings: Vec::new(),
        };

        #[cfg(feature = "typst")]
        {
            engine.register_backend(Box::new(quillmark_typst::TypstBackend));
        }

        engine
    }

    /// Register a backend with the engine.
    pub fn register_backend(&mut self, backend: Box<dyn Backend>) {
        let id = backend.id().to_string();
        self.backends.insert(id, Arc::from(backend));
    }

    /// Returns all currently accumulated non-fatal engine warnings.
    pub fn warnings(&self) -> &[Diagnostic] {
        &self.warnings
    }

    /// Drains and returns all currently accumulated non-fatal engine warnings.
    pub fn take_warnings(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.warnings)
    }

    /// Load a quill from a file tree and attach the appropriate backend.
    pub fn load_quill(&self, tree: quillmark_core::FileTreeNode) -> Result<Quill, RenderError> {
        let quill = Quill::from_tree(tree).map_err(|e| RenderError::QuillConfig {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!("Failed to load quill from tree: {}", e),
                )
                .with_code("quill::load_failed".to_string()),
            ),
        })?;
        self.attach_backend(quill)
    }

    /// Load a quill from a filesystem path and attach the appropriate backend.
    pub fn quill_from_path<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<Quill, RenderError> {
        let quill = Quill::from_path(path).map_err(|e| RenderError::QuillConfig {
            diag: Box::new(
                Diagnostic::new(Severity::Error, format!("Failed to load quill: {}", e))
                    .with_code("quill::load_failed".to_string()),
            ),
        })?;
        self.attach_backend(quill)
    }

    fn attach_backend(&self, quill: Quill) -> Result<Quill, RenderError> {
        let backend_id = quill.backend_id.as_str();
        let backend =
            self.backends
                .get(backend_id)
                .ok_or_else(|| RenderError::UnsupportedBackend {
                    diag: Box::new(
                        Diagnostic::new(
                            Severity::Error,
                            format!("Backend '{}' not registered or not enabled", backend_id),
                        )
                        .with_code("engine::backend_not_found".to_string())
                        .with_hint(format!(
                            "Available backends: {}",
                            self.backends.keys().cloned().collect::<Vec<_>>().join(", ")
                        )),
                    ),
                })?;
        Ok(quill.with_backend(Arc::clone(backend)))
    }

    /// Create a workflow for rendering with the given quill.
    ///
    /// The quill's `backend_id` is looked up in the engine's registered backends.
    /// Use `load_quill()` or `quill_from_path()` to get a quill with backend attached,
    /// or pass a quill loaded directly if its backend is registered.
    pub fn workflow(&self, quill: &Quill) -> Result<Workflow, RenderError> {
        let backend_id = quill.backend_id.as_str();
        let backend =
            self.backends
                .get(backend_id)
                .ok_or_else(|| RenderError::UnsupportedBackend {
                    diag: Box::new(
                        Diagnostic::new(
                            Severity::Error,
                            format!("Backend '{}' not registered or not enabled", backend_id),
                        )
                        .with_code("engine::backend_not_found".to_string())
                        .with_hint(format!(
                            "Available backends: {}",
                            self.backends.keys().cloned().collect::<Vec<_>>().join(", ")
                        )),
                    ),
                })?;
        Workflow::new(Arc::clone(backend), quill.clone())
    }

    /// Get a list of registered backend IDs.
    pub fn registered_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}
