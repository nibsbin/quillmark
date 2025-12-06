use quillmark_core::{Backend, Diagnostic, Plate, RenderError, Severity};
use std::collections::HashMap;
use std::sync::Arc;

use super::workflow::Workflow;
use super::PlateRef;

/// High-level engine for orchestrating backends and plates. See [module docs](super) for usage patterns.
pub struct Quillmark {
    backends: HashMap<String, Arc<dyn Backend>>,
    plates: HashMap<String, Plate>,
}

impl Quillmark {
    /// Create a new Quillmark with auto-registered backends based on enabled features.
    pub fn new() -> Self {
        let mut engine = Self {
            backends: HashMap::new(),
            plates: HashMap::new(),
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
    /// If the backend provides a default Plate and no Plate named `__default__`
    /// is already registered, the default Plate will be automatically registered.
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
    /// #     fn glue_extension_types(&self) -> &'static [&'static str] { &[".custom"] }
    /// #     fn allow_auto_glue(&self) -> bool { true }
    /// #     fn register_filters(&self, _: &mut quillmark_core::Glue) {}
    /// #     fn compile(&self, _: &str, _: &quillmark_core::Plate, _: &quillmark_core::RenderOptions) -> Result<quillmark_core::RenderResult, quillmark_core::RenderError> {
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

        // Get default Plate before moving backend
        let default_plate = backend.default_plate();

        // Register backend first so it's available when registering default Plate
        self.backends.insert(id.clone(), Arc::from(backend));

        // Register default Plate if available and not already registered
        if !self.plates.contains_key("__default__") {
            if let Some(default_plate) = default_plate {
                if let Err(e) = self.register_plate(default_plate) {
                    eprintln!(
                        "Warning: Failed to register default Plate from backend '{}': {}",
                        id, e
                    );
                }
            }
        }
    }

    /// Register a plate template with the engine by name.
    ///
    /// Validates the plate configuration against the registered backend, including:
    /// - Backend exists and is registered
    /// - Glue file extension matches backend requirements
    /// - Auto-glue is allowed if no glue file is specified
    /// - Plate name is unique
    pub fn register_plate(&mut self, plate: Plate) -> Result<(), RenderError> {
        let name = plate.name.clone();

        // Check name uniqueness
        if self.plates.contains_key(&name) {
            return Err(RenderError::PlateConfig {
                diag: Diagnostic::new(
                    Severity::Error,
                    format!("Plate '{}' is already registered", name),
                )
                .with_code("quill::name_collision".to_string())
                .with_hint("Each plate must have a unique name".to_string()),
            });
        }

        // Get backend
        let backend_id = plate.backend.as_str();
        let backend = self
            .backends
            .get(backend_id)
            .ok_or_else(|| RenderError::PlateConfig {
                diag: Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Backend '{}' specified in plate '{}' is not registered",
                        backend_id, name
                    ),
                )
                .with_code("quill::backend_not_found".to_string())
                .with_hint(format!(
                    "Available backends: {}",
                    self.backends.keys().cloned().collect::<Vec<_>>().join(", ")
                )),
            })?;

        // Validate glue_file extension or auto_glue
        if let Some(glue_file) = &plate.metadata.get("glue_file").and_then(|v| v.as_str()) {
            let extension = std::path::Path::new(glue_file)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();

            if !backend.glue_extension_types().contains(&extension.as_str()) {
                return Err(RenderError::PlateConfig {
                    diag: Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Glue file '{}' has extension '{}' which is not supported by backend '{}'",
                            glue_file, extension, backend_id
                        ),
                    )
                    .with_code("quill::glue_extension_mismatch".to_string())
                    .with_hint(format!(
                        "Supported extensions for '{}' backend: {}",
                        backend_id,
                        backend.glue_extension_types().join(", ")
                    )),
                });
            }
        } else {
            if !backend.allow_auto_glue() {
                return Err(RenderError::PlateConfig {
                    diag: Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Backend '{}' does not support automatic glue generation, but plate '{}' does not specify a glue file",
                            backend_id, name
                        ),
                    )
                    .with_code("quill::auto_glue_not_allowed".to_string())
                    .with_hint(format!(
                        "Add a glue file with one of these extensions: {}",
                        backend.glue_extension_types().join(", ")
                    )),
                });
            }
        }

        self.plates.insert(name, plate);
        Ok(())
    }

    /// Load a workflow by plate reference (name, object, or parsed document)
    ///
    /// This is the unified workflow creation method that accepts:
    /// - `&str` - Looks up registered plate by name
    /// - `&Plate` - Uses plate directly (doesn't need to be registered)
    /// - `&ParsedDocument` - Extracts plate tag and looks up by name
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Plate, ParsedDocument};
    /// # let engine = Quillmark::new();
    /// // By name
    /// let workflow = engine.workflow("my-plate")?;
    ///
    /// // By object
    /// # let plate = Plate::from_path("path/to/plate").unwrap();
    /// let workflow = engine.workflow(&plate)?;
    ///
    /// // From parsed document
    /// # let parsed = ParsedDocument::from_markdown("---\nQUILL: my-plate\n---\n# Hello").unwrap();
    /// let workflow = engine.workflow(&parsed)?;
    /// # Ok::<(), quillmark::RenderError>(())
    /// ```
    pub fn workflow<'a>(
        &self,
        plate_ref: impl Into<PlateRef<'a>>,
    ) -> Result<Workflow, RenderError> {
        let plate_ref = plate_ref.into();

        // Get the plate reference based on the parameter type
        let plate = match plate_ref {
            PlateRef::Name(name) => {
                // Look up the plate by name
                self.plates
                    .get(name)
                    .ok_or_else(|| RenderError::UnsupportedBackend {
                        diag: Diagnostic::new(
                            Severity::Error,
                            format!("Plate '{}' not registered", name),
                        )
                        .with_code("engine::plate_not_found".to_string())
                        .with_hint(format!(
                            "Available plates: {}",
                            self.plates.keys().cloned().collect::<Vec<_>>().join(", ")
                        )),
                    })?
            }
            PlateRef::Object(plate) => {
                // Use the provided plate directly
                plate
            }
            PlateRef::Parsed(parsed) => {
                // Extract plate tag from parsed document and look up by name
                let plate_tag = parsed.quill_tag();
                self.plates
                    .get(plate_tag)
                    .ok_or_else(|| RenderError::UnsupportedBackend {
                        diag: Diagnostic::new(
                            Severity::Error,
                            format!("Plate '{}' not registered", plate_tag),
                        )
                        .with_code("engine::plate_not_found".to_string())
                        .with_hint(format!(
                            "Available plates: {}",
                            self.plates.keys().cloned().collect::<Vec<_>>().join(", ")
                        )),
                    })?
            }
        };

        // Get backend ID from plate metadata
        let backend_id = plate
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RenderError::EngineCreation {
                diag: Diagnostic::new(
                    Severity::Error,
                    format!("Plate '{}' does not specify a backend", plate.name),
                )
                .with_code("engine::missing_backend".to_string())
                .with_hint(
                    "Add 'backend = \"typst\"' to the [Plate] section of Plate.toml".to_string(),
                ),
            })?;

        // Get the backend by ID
        let backend =
            self.backends
                .get(backend_id)
                .ok_or_else(|| RenderError::UnsupportedBackend {
                    diag: Diagnostic::new(
                        Severity::Error,
                        format!("Backend '{}' not registered or not enabled", backend_id),
                    )
                    .with_code("engine::backend_not_found".to_string())
                    .with_hint(format!(
                        "Available backends: {}",
                        self.backends.keys().cloned().collect::<Vec<_>>().join(", ")
                    )),
                })?;

        // Clone the Arc reference to the backend and the plate for the workflow
        let backend_clone = Arc::clone(backend);
        let plate_clone = plate.clone();

        Workflow::new(backend_clone, plate_clone)
    }

    /// Get a list of registered backend IDs.
    pub fn registered_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// Get a list of registered plate names.
    pub fn registered_quills(&self) -> Vec<&str> {
        self.plates.keys().map(|s| s.as_str()).collect()
    }

    /// Get a reference to a registered plate by name.
    ///
    /// Returns `None` if the plate is not registered.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let engine = Quillmark::new();
    /// if let Some(plate) = engine.get_quill("my-plate") {
    ///     println!("Found plate: {}", plate.name);
    /// }
    /// ```
    pub fn get_quill(&self, name: &str) -> Option<&Plate> {
        self.plates.get(name)
    }

    /// Get a reference to a plate's metadata by name.
    ///
    /// Returns `None` if the plate is not registered.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let engine = Quillmark::new();
    /// if let Some(metadata) = engine.get_quill_metadata("my-plate") {
    ///     println!("Metadata: {:?}", metadata);
    /// }
    /// ```
    pub fn get_quill_metadata(
        &self,
        name: &str,
    ) -> Option<&HashMap<String, quillmark_core::value::QuillValue>> {
        self.plates.get(name).map(|plate| &plate.metadata)
    }

    /// Unregister a plate by name.
    ///
    /// Returns `true` if the plate was registered and has been removed,
    /// `false` if the plate was not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::Quillmark;
    /// # let mut engine = Quillmark::new();
    /// if engine.unregister_plate("my-plate") {
    ///     println!("Plate unregistered");
    /// }
    /// ```
    pub fn unregister_plate(&mut self, name: &str) -> bool {
        self.plates.remove(name).is_some()
    }
}

impl Default for Quillmark {
    fn default() -> Self {
        Self::new()
    }
}
