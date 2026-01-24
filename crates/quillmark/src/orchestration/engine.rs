use quillmark_core::{
    Backend, Diagnostic, Quill, QuillReference, RenderError, Severity, Version, VersionSelector,
};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;

use super::workflow::Workflow;
use super::QuillRef;

/// A set of versioned Quills for a given template name
struct VersionedQuillSet {
    /// Map of version to Quill, sorted by version
    versions: BTreeMap<Version, Quill>,
}

impl VersionedQuillSet {
    /// Create a new empty VersionedQuillSet
    fn new() -> Self {
        Self {
            versions: BTreeMap::new(),
        }
    }

    /// Add a version to the set
    fn insert(&mut self, version: Version, quill: Quill) {
        self.versions.insert(version, quill);
    }

    /// Resolve a version selector to a specific version
    fn resolve(&self, selector: &VersionSelector) -> Option<&Quill> {
        match selector {
            VersionSelector::Exact(v) => self.versions.get(v),
            VersionSelector::Major(major) => {
                // Find the highest minor version with matching major
                self.versions
                    .iter()
                    .rev()
                    .find(|(v, _)| v.major == *major)
                    .map(|(_, quill)| quill)
            }
            VersionSelector::Latest => {
                // Return the highest version overall
                let result = self.versions.iter().next_back().map(|(_, quill)| quill);
                debug_assert!(
                    result.is_some(),
                    "VersionedQuillSet should never be empty - quills must have at least one version"
                );
                result
            }
        }
    }

    /// Get all available versions, sorted in descending order
    fn available_versions(&self) -> Vec<Version> {
        self.versions.keys().rev().copied().collect()
    }
}

/// High-level engine for orchestrating backends and quills. See [module docs](super) for usage patterns.
pub struct Quillmark {
    backends: HashMap<String, Arc<dyn Backend>>,
    quills: HashMap<String, VersionedQuillSet>,
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
    /// #     fn compile(&self, _: &str, _: &quillmark_core::Quill, _: &quillmark_core::RenderOptions, _: &serde_json::Value) -> Result<quillmark_core::RenderResult, quillmark_core::RenderError> {
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
    /// - Version is valid (MAJOR.MINOR format)
    ///
    /// Multiple versions of the same quill can be registered.
    pub fn register_quill(&mut self, quill: Quill) -> Result<(), RenderError> {
        let name = quill.name.clone();

        // Extract and validate version from quill metadata
        let version_str = quill
            .metadata
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RenderError::InvalidVersion {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!("Quill '{}' is missing required 'version' field", name),
                    )
                    .with_code("quill::missing_version".to_string())
                    .with_hint(
                        "Add 'version = \"1.0\"' to the [Quill] section of Quill.yaml".to_string(),
                    ),
                ),
            })?;

        let version = Version::from_str(version_str).map_err(|e| RenderError::InvalidVersion {
            diag: Box::new(
                Diagnostic::new(
                    Severity::Error,
                    format!(
                        "Quill '{}' has invalid version '{}': {}",
                        name, version_str, e
                    ),
                )
                .with_code("quill::invalid_version".to_string())
                .with_hint("Version must be in MAJOR.MINOR format (e.g., '2.1')".to_string()),
            ),
        })?;

        // Check if this exact version is already registered
        if let Some(version_set) = self.quills.get(&name) {
            if version_set.versions.contains_key(&version) {
                return Err(RenderError::QuillConfig {
                    diag: Box::new(
                        Diagnostic::new(
                            Severity::Error,
                            format!("Quill '{}' version {} is already registered", name, version),
                        )
                        .with_code("quill::version_collision".to_string())
                        .with_hint("Each version of a quill must be unique".to_string()),
                    ),
                });
            }
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
        }

        // Add the quill to the versioned set
        self.quills
            .entry(name.clone())
            .or_insert_with(VersionedQuillSet::new)
            .insert(version, quill);

        Ok(())
    }

    /// Resolve a QuillReference to a specific Quill
    fn resolve_quill_reference(&self, quill_ref: &QuillReference) -> Result<&Quill, RenderError> {
        let version_set =
            self.quills
                .get(&quill_ref.name)
                .ok_or_else(|| RenderError::QuillNotFound {
                    diag: Box::new(
                        Diagnostic::new(
                            Severity::Error,
                            format!("Quill '{}' not registered", quill_ref.name),
                        )
                        .with_code("engine::quill_not_found".to_string())
                        .with_hint(format!(
                            "Available quills: {}",
                            self.quills.keys().cloned().collect::<Vec<_>>().join(", ")
                        )),
                    ),
                })?;

        version_set.resolve(&quill_ref.selector).ok_or_else(|| {
            let available = version_set.available_versions();
            let available_str: Vec<String> = available.iter().map(|v| v.to_string()).collect();

            let suggestion = match &quill_ref.selector {
                VersionSelector::Exact(v) => {
                    // Suggest using major version or listing available versions
                    if let Some(latest_major) = available.iter().find(|av| av.major == v.major) {
                        format!(
                            "Use @{} for latest {}.x (currently {}), or specify one of: {}",
                            v.major,
                            v.major,
                            latest_major,
                            available_str.join(", ")
                        )
                    } else {
                        format!("Available versions: {}", available_str.join(", "))
                    }
                }
                VersionSelector::Major(m) => {
                    format!(
                        "No versions found in {}.x series. Available versions: {}",
                        m,
                        available_str.join(", ")
                    )
                }
                VersionSelector::Latest => {
                    "No versions available (this should not happen)".to_string()
                }
            };

            RenderError::VersionNotFound {
                diag: Box::new(
                    Diagnostic::new(
                        Severity::Error,
                        format!(
                            "Version not found for quill '{}' with selector '{}'",
                            quill_ref.name, quill_ref.selector
                        ),
                    )
                    .with_code("engine::version_not_found".to_string())
                    .with_hint(suggestion),
                ),
            }
        })
    }

    /// Load a workflow by quill reference (name, object, or parsed document)
    ///
    /// This is the unified workflow creation method that accepts:
    /// - `&str` - Looks up registered quill by name (uses latest version)
    /// - `&Quill` - Uses quill directly (doesn't need to be registered)
    /// - `&ParsedDocument` - Extracts quill reference and resolves version
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use quillmark::{Quillmark, Quill, ParsedDocument};
    /// # let engine = Quillmark::new();
    /// // By name (uses latest version)
    /// let workflow = engine.workflow("my-quill")?;
    ///
    /// // By object
    /// # let quill = Quill::from_path("path/to/quill").unwrap();
    /// let workflow = engine.workflow(&quill)?;
    ///
    /// // From parsed document (with version resolution)
    /// # let parsed = ParsedDocument::from_markdown("---\nQUILL: my-quill@2.1\n---\n# Hello").unwrap();
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
                // Parse the name as a QuillReference (supports @version syntax)
                let quill_reference =
                    QuillReference::from_str(name).map_err(|e| RenderError::InvalidVersion {
                        diag: Box::new(
                            Diagnostic::new(
                                Severity::Error,
                                format!("Invalid quill reference '{}': {}", name, e),
                            )
                            .with_code("engine::invalid_reference".to_string()),
                        ),
                    })?;
                self.resolve_quill_reference(&quill_reference)?
            }
            QuillRef::Object(quill) => {
                // Use the provided quill directly
                quill
            }
            QuillRef::Parsed(parsed) => {
                // Extract quill reference from parsed document and resolve version
                let quill_reference = parsed.quill_reference();
                self.resolve_quill_reference(quill_reference)?
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
                        "Add 'backend = \"typst\"' to the [Quill] section of Quill.yaml"
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

    /// Get a reference to a registered quill by name (returns latest version).
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
        self.quills
            .get(name)
            .and_then(|vs| vs.resolve(&VersionSelector::Latest))
    }

    /// Get a reference to a quill's metadata by name (returns latest version).
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
        self.get_quill(name).map(|quill| &quill.metadata)
    }

    /// Unregister all versions of a quill by name.
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
