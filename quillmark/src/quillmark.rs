//! High-level engine for orchestrating backends and quills. See [module docs](self) for usage patterns.

#![doc = include_str!("../docs/quillmark.md")]

use crate::Workflow;
use quillmark_core::{Backend, Quill, RenderError};
use std::collections::HashMap;

/// Ergonomic reference to a Quill by name or object.
pub enum QuillRef<'a> {
    /// Reference to a quill by its registered name
    Name(&'a str),
    /// Reference to a borrowed Quill object
    Object(&'a Quill),
}

impl<'a> From<&'a Quill> for QuillRef<'a> {
    fn from(quill: &'a Quill) -> Self {
        QuillRef::Object(quill)
    }
}

impl<'a> From<&'a str> for QuillRef<'a> {
    fn from(name: &'a str) -> Self {
        QuillRef::Name(name)
    }
}

impl<'a> From<&'a String> for QuillRef<'a> {
    fn from(name: &'a String) -> Self {
        QuillRef::Name(name.as_str())
    }
}

impl<'a> From<&'a std::borrow::Cow<'a, str>> for QuillRef<'a> {
    fn from(name: &'a std::borrow::Cow<'a, str>) -> Self {
        QuillRef::Name(name.as_ref())
    }
}

/// High-level engine for orchestrating backends and quills. See [module docs](self) for usage patterns.
pub struct Quillmark {
    backends: HashMap<String, Box<dyn Backend>>,
    quills: HashMap<String, Quill>,
}

impl Quillmark {
    /// Create a new Quillmark engine with auto-registered backends based on enabled features.
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut backends: HashMap<String, Box<dyn Backend>> = HashMap::new();

        // Auto-register backends based on enabled features
        #[cfg(feature = "typst")]
        {
            let backend = Box::new(quillmark_typst::TypstBackend::default());
            backends.insert(backend.id().to_string(), backend);
        }

        Self {
            backends,
            quills: HashMap::new(),
        }
    }

    /// Register a quill template with the engine by name.
    pub fn register_quill(&mut self, quill: Quill) {
        let name = quill.name.clone();
        self.quills.insert(name, quill);
    }

    /// Load a workflow by quill name or object reference. See [module docs](self) for examples.
    pub fn load<'a>(&self, quill_ref: impl Into<QuillRef<'a>>) -> Result<Workflow, RenderError> {
        let quill_ref = quill_ref.into();

        // Get the quill reference based on the parameter type
        let quill = match quill_ref {
            QuillRef::Name(name) => {
                // Look up the quill by name
                self.quills.get(name).ok_or_else(|| {
                    RenderError::Other(format!("Quill '{}' not registered", name).into())
                })?
            }
            QuillRef::Object(quill) => {
                // Use the provided quill directly
                quill
            }
        };

        // Get backend ID from quill metadata
        let backend_id = quill
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                RenderError::Other(
                    format!("Quill '{}' does not specify a backend", quill.name).into(),
                )
            })?;

        // Get the backend by ID
        let backend = self.backends.get(backend_id).ok_or_else(|| {
            RenderError::Other(
                format!("Backend '{}' not registered or not enabled", backend_id).into(),
            )
        })?;

        // Clone the backend and quill for the workflow
        // Note: We need to box clone the backend trait object
        let backend_clone = self.clone_backend(backend.as_ref());
        let quill_clone = quill.clone();

        Workflow::new(backend_clone, quill_clone)
    }

    /// Helper method to clone a backend (trait object cloning workaround)
    fn clone_backend(&self, backend: &dyn Backend) -> Box<dyn Backend> {
        // For each backend, we need to instantiate a new one
        // This is a workaround since we can't clone trait objects directly
        match backend.id() {
            #[cfg(feature = "typst")]
            "typst" => Box::new(quillmark_typst::TypstBackend::default()),
            _ => panic!("Unknown backend: {}", backend.id()),
        }
    }

    /// Get a list of registered backend IDs.
    pub fn registered_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// Get a list of registered quill names.
    pub fn registered_quills(&self) -> Vec<&str> {
        self.quills.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}
