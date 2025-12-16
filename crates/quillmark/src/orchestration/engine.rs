use quillmark_core::{Backend, Diagnostic, Quill, RenderError, Severity};
use std::collections::HashMap;
use std::sync::Arc;

use super::workflow::Workflow;
use super::QuillRef;

/// High-level engine for orchestrating backends and quills. See [module docs](super) for usage patterns.
pub struct Quillmark {
    backends: HashMap<String, Arc<dyn Backend>>,
    quills: HashMap<String, Quill>,
}

impl Quillmark {
    /// Create a new Quillmark with auto-registered backends based on enabled features.
    pub fn new() -> Self {
        let mut engine = Self {
            backends: HashMap::new(),
            quills: HashMap::new(),
        };

        // Auto-register backends based on enabled features
        #[cfg(feature = "typst")]
        {
            engine.register_backend(Box::new(quillmark_typst::TypstBackend));
        }

        #[cfg(feature = "acroform")]
        {
            engine.register_backend(Box::new(quillmark_acroform::AcroformBackend));
        }

        engine
    }

    /// Register a backend with the engine.
    ///
    /// This method allows registering custom backends or explicitly registering
    /// feature-integrated backends. The backend is registered by its ID.
    ///
    /// If the backend provides a default Quill and no Quill named `__default__`
    /// is already registered, the default Quill will be automatically registered.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use quillmark::Quillmark;
    /// # use quillmark_core::Backend;
    /// # struct CustomBackend;
    /// # impl Backend for CustomBackend {
    /// #     fn id(&self) -> &'static str { "custom" }
    /// #     fn supported_formats(&self) -> &'static [quillmark_core::OutputFormat] { &[] }
    /// #     fn plate_extension_types(&self) -> &'static [&'static str] { &[".custom"] }
    /// #     fn allow_auto_plate(&self) -> bool { true }
    /// #     fn register_filters(&self, _: &mut quillmark_core::Plate) {}
    /// #     fn compile(&self, _: &str, _: &quillmark_core::Quill, _: &quillmark_core::RenderOptions) -> Result<quillmark_core::RenderResult, quillmark_core::RenderError> {
    /// #         Ok(quillmark_core::RenderResult::new(vec![], quillmark_core::OutputFormat::Txt))
    /// #     }
    /// # }
    ///
    /// let mut engine = Quillmark::new();
    /// let custom_backend = Box::new(CustomBackend);
    /// engine.register_backend(custom_backend);
    /// ```
    pub fn register_backend(&mut self, backend: Box<dyn Backend>) {
        let id = backend.id().to_string();

        // Get default Quill before moving backend
        let default_quill = backend.default_quill();

        // Register backend first so it's available when registering default Quill
        self.backends.insert(id.clone(), Arc::from(backend));

        // Register default Quill if available and not already registered
        if !self.quills.contains_key("__default__") {
            if let Some(default_quill) = default_quill {
                if let Err(e) = self.register_quill(default_quill) {
                    eprintln!(
                        "Warning: Failed to register default Quill from backend '{}': {}",
                        id, e
                    );
                }
            }
        }
    }

    /// Register a quill template with the engine by name.
    ///
    /// Validates the quill configuration against the registered backend, including:
    /// - Backend exists and is registered
    /// - Plate file extension matches backend requirements
    /// - Auto-plate is allowed if no plate file is specified
    /// - Quill name is unique
    pub fn register_quill(&mut self, quill: Quill) -> Result<(), RenderError> {
        let name = quill.name.clone();

        // Check name uniqueness
        if self.quills.contains_key(&name) {
            return Err(RenderError::QuillConfig {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!("Quill '{}' is already registered", name),
                    )
                    .with_code("quill::name_collision".to_string())
                    .with_hint("Each quill must have a unique name".to_string()),
                ),
            });
        }

        // Get backend
        let backend_id = quill.backend.as_str();
        let backend = self
            .backends
            .get(backend_id)
            .ok_or_else(|| RenderError::QuillConfig {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Backend '{}' specified in quill '{}' is not registered",
                            backend_id, name
                        ),
                    )
                    .with_code("quill::backend_not_found".to_string())
                    .with_hint(format!(
                        "Available backends: {}",
                        self.backends.keys().cloned().collect::<Vec<_>>().join(", ")
                    )),
                ),
            })?;

        // Validate plate_file extension or auto_plate
        if let Some(plate_file) = &quill.metadata.get("plate_file").and_then(|v| v.as_str()) {
            let extension = std::path::Path::new(plate_file)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();

            if !backend
                .plate_extension_types()
                .contains(&extension.as_str())
            {
                return Err(RenderError::QuillConfig {
                    diag: Box::new(Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Plate file '{}' has extension '{}' which is not supported by backend '{}'",
                            plate_file, extension, backend_id
                        ),
                    )
                    .with_code("quill::plate_extension_mismatch".to_string())
                    .with_hint(format!(
                        "Supported extensions for '{}' backend: {}",
                        backend_id,
                        backend.plate_extension_types().join(", ")
                    ))),
                });
            }
        } else if !backend.allow_auto_plate() {
            return Err(RenderError::QuillConfig {
                diag: Box::new(Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Backend '{}' does not support automatic plate generation, but quill '{}' does not specify a plate file",
                        backend_id, name
                    ),
                )
                .with_code("quill::auto_plate_not_allowed".to_string())
                .with_hint(format!(
                    "Add a plate file with one of these extensions: {}",
                    backend.plate_extension_types().join(", ")
                ))),
            });
        }

        self.quills.insert(name, quill);
        Ok(())
    }

    /// Load a workflow by quill reference (name, object, or parsed document)
    ///
    /// This is the unified workflow creation method that accepts:
    /// - `&str` - Looks up registered quill by name
    /// - `&Quill` - Uses quill directly (doesn't need to be registered)
    /// - `&ParsedDocument` - Extracts quill tag and looks up by name
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Quill, ParsedDocument};
    /// # let engine = Quillmark::new();
    /// // By name
    /// let workflow = engine.workflow("my-quill")?;
    ///
    /// // By object
    /// # let quill = Quill::from_path("path/to/quill").unwrap();
    /// let workflow = engine.workflow(&quill)?;
    ///
    /// // From parsed document
    /// # let parsed = ParsedDocument::from_markdown("---\nQUILL: my-quill\n---\n# Hello").unwrap();
    /// let workflow = engine.workflow(&parsed)?;
    /// # Ok::<(), quillmark::RenderError>(())
    /// ```
    pub fn workflow<'a>(
        &self,
        quill_ref: impl Into<QuillRef<'a>>,
    ) -> Result<Workflow, RenderError> {
        let quill_ref = quill_ref.into();

        // Get the quill reference based on the parameter type
        let quill = match quill_ref {
            QuillRef::Name(name) => {
                // Look up the quill by name
                self.quills
                    .get(name)
                    .ok_or_else(|| RenderError::UnsupportedBackend {
                        diag: Box::new(
                            Diagnostic::new(
                                Severity::Error,
                                format!("Quill '{}' not registered", name),
                            )
                            .with_code("engine::quill_not_found".to_string())
                            .with_hint(format!(
                                "Available quills: {}",
                                self.quills.keys().cloned().collect::<Vec<_>>().join(", ")
                            )),
                        ),
                    })?
            }
            QuillRef::Object(quill) => {
                // Use the provided quill directly
                quill
            }
            QuillRef::Parsed(parsed) => {
                // Extract quill tag from parsed document and look up by name
                let quill_tag = parsed.quill_tag();
                self.quills
                    .get(quill_tag)
                    .ok_or_else(|| RenderError::UnsupportedBackend {
                        diag: Box::new(
                            Diagnostic::new(
                                Severity::Error,
                                format!("Quill '{}' not registered", quill_tag),
                            )
                            .with_code("engine::quill_not_found".to_string())
                            .with_hint(format!(
                                "Available quills: {}",
                                self.quills.keys().cloned().collect::<Vec<_>>().join(", ")
                            )),
                        ),
                    })?
            }
        };

        // Get backend ID from quill metadata
        let backend_id = quill
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RenderError::EngineCreation {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!("Quill '{}' does not specify a backend", quill.name),
                    )
                    .with_code("engine::missing_backend".to_string())
                    .with_hint(
                        "Add 'backend = \"typst\"' to the [Quill] section of Quill.toml"
                            .to_string(),
                    ),
                ),
            })?;

        // Get the backend by ID
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

        // Clone the Arc reference to the backend and the quill for the workflow
        let backend_clone = Arc::clone(backend);
        let quill_clone = quill.clone();

        Workflow::new(backend_clone, quill_clone)
    }

    /// Get a list of registered backend IDs.
    pub fn registered_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// Get a list of registered quill names.
    pub fn registered_quills(&self) -> Vec<&str> {
        self.quills.keys().map(|s| s.as_str()).collect()
    }

    /// Get a reference to a registered quill by name.
    ///
    /// Returns `None` if the quill is not registered.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let engine = Quillmark::new();
    /// if let Some(quill) = engine.get_quill("my-quill") {
    ///     println!("Found quill: {}", quill.name);
    /// }
    /// ```
    pub fn get_quill(&self, name: &str) -> Option<&Quill> {
        self.quills.get(name)
    }

    /// Get a reference to a quill's metadata by name.
    ///
    /// Returns `None` if the quill is not registered.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let engine = Quillmark::new();
    /// if let Some(metadata) = engine.get_quill_metadata("my-quill") {
    ///     println!("Metadata: {:?}", metadata);
    /// }
    /// ```
    pub fn get_quill_metadata(
        &self,
        name: &str,
    ) -> Option<&HashMap<String, quillmark_core::value::QuillValue>> {
        self.quills.get(name).map(|quill| &quill.metadata)
    }

    /// Unregister a quill by name.
    ///
    /// Returns `true` if the quill was registered and has been removed,
    /// `false` if the quill was not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let mut engine = Quillmark::new();
    /// if engine.unregister_quill("my-quill") {
    ///     println!("Quill unregistered");
    /// }
    /// ```
    pub fn unregister_quill(&mut self, name: &str) -> bool {
        self.quills.remove(name).is_some()
    }
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}
