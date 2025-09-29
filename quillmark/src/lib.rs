// Re-export all types from quillmark-core for backward compatibility
pub use quillmark_core::{Artifact, Backend, OutputFormat, RenderError, Quill, Glue, RenderOptions};
use std::path::{Path, PathBuf};

/// Result type for rendering operations
pub type RenderResult = Result<Vec<Artifact>, RenderError>;

/// Sealed QuillEngine struct - the primary API for rendering markdown documents
///
/// This struct encapsulates a backend and a quill template, providing the single
/// authoritative interface for all rendering operations. All rendering functionality
/// is contained within this struct to provide a highly opinionated, consistent API.
pub struct QuillEngine {
    backend: Box<dyn Backend>,
    quill: Quill,
}

impl QuillEngine {
    /// Create a new QuillEngine with the given backend and quill template
    ///
    /// This is the only way to create a QuillEngine. The engine loads and validates
    /// the quill template from the filesystem and prepares it for rendering.
    ///
    /// # Arguments
    /// * `backend` - The backend implementation to use for compilation
    /// * `quill_path` - Path to the quill template directory
    ///
    /// # Returns
    /// A new QuillEngine instance ready for rendering
    ///
    /// # Errors
    /// Returns `RenderError` if:
    /// - The quill path doesn't exist
    /// - The quill template is invalid or missing required files
    /// - The backend is incompatible with the quill template
    pub fn new(backend: Box<dyn Backend>, quill_path: PathBuf) -> Result<Self, RenderError> {
        // Load and validate quill template
        let quill = load_quill_from_path(&quill_path, backend.glue_type())?;
        
        Ok(QuillEngine {
            backend,
            quill,
        })
    }
    
    /// Render markdown content using the configured backend and quill template
    ///
    /// This is the primary rendering method that:
    /// 1. Parses YAML frontmatter and markdown content
    /// 2. Creates template glue and registers backend filters  
    /// 3. Renders the template with parsed context
    /// 4. Compiles final artifacts using the backend
    ///
    /// # Arguments
    /// * `markdown` - The markdown content with optional YAML frontmatter
    ///
    /// # Returns
    /// A vector of artifacts produced by the backend
    ///
    /// # Errors
    /// Returns `RenderError` if:
    /// - YAML frontmatter parsing fails
    /// - Template rendering fails
    /// - Backend compilation fails
    pub fn render(&self, markdown: &str) -> RenderResult {
        self.render_with_format(markdown, None)
    }
    
    /// Render markdown content with a specific output format
    ///
    /// This method is like `render()` but allows specifying a particular output format.
    /// If the backend doesn't support the requested format, an error is returned.
    ///
    /// # Arguments
    /// * `markdown` - The markdown content with optional YAML frontmatter
    /// * `format` - The desired output format
    ///
    /// # Returns
    /// A vector of artifacts in the specified format
    ///
    /// # Errors
    /// Returns `RenderError` if the backend doesn't support the format or other rendering errors occur
    pub fn render_with_format(&self, markdown: &str, format: Option<OutputFormat>) -> RenderResult {
        // Validate format is supported if specified
        if let Some(fmt) = format {
            if !self.backend.supported_formats().contains(&fmt) {
                return Err(RenderError::FormatNotSupported {
                    backend: self.backend.id().to_string(),
                    format: fmt,
                });
            }
        }
        
        // Parse markdown to extract frontmatter and body
        let parsed_doc = quillmark_core::decompose(markdown)
            .map_err(|e| RenderError::Other(format!("Failed to parse markdown: {}", e).into()))?;
        
        // Create Glue instance with the template and register backend filters
        let mut glue = Glue::new(self.quill.template_content.clone());
        
        // Register filters from the backend
        self.backend.register_filters(&mut glue);
        
        // Render the template with the parsed document context
        let glue_content = glue.compose(parsed_doc.fields().clone())
            .map_err(|e| RenderError::Other(Box::new(e)))?;
        
        // Call the backend to compile the final artifacts
        let render_options = RenderOptions {
            output_format: format,
        };
        self.backend.compile(&glue_content, &self.quill, &render_options)
    }
    
    /// Get the backend identifier
    ///
    /// # Returns
    /// The backend's stable identifier string
    pub fn backend_id(&self) -> &str {
        self.backend.id()
    }
    
    /// Get the output formats supported by the backend
    ///
    /// # Returns
    /// A slice of supported output formats
    pub fn supported_formats(&self) -> &[OutputFormat] {
        self.backend.supported_formats()
    }
    
    /// Get the name of the loaded quill template
    ///
    /// # Returns
    /// The quill template name
    pub fn quill_name(&self) -> &str {
        &self.quill.name
    }
    
    /// Get the glue file extension used by the backend
    ///
    /// # Returns
    /// The file extension (e.g., ".typ", ".tex")
    pub fn glue_type(&self) -> &str {
        self.backend.glue_type()
    }
}

/// Legacy render function for backward compatibility
///
/// This function is deprecated. Use `QuillEngine::new()` followed by `engine.render()` instead.
/// 
/// # Deprecation Notice
/// This function will be removed in a future version. Migrate to the new QuillEngine API:
/// ```rust
/// // Old way (deprecated)
/// let config = RenderConfig { backend, output_format: None, quill_path };
/// let result = render(markdown, &config)?;
/// 
/// // New way (recommended)
/// let engine = QuillEngine::new(backend, quill_path)?;
/// let result = engine.render(markdown)?;
/// ```
#[deprecated(since = "0.2.0", note = "Use QuillEngine::new() and engine.render() instead")]
pub fn render(markdown: &str, config: &LegacyRenderConfig) -> RenderResult {
    // Note: This is a minimal implementation that doesn't support all legacy features
    // For full functionality, please migrate to QuillEngine API
    let quill = load_quill_from_path(&config.quill_path, ".typ")?; // Default to .typ for legacy
    let engine_config = RenderOptions {
        output_format: config.output_format,
    };
    
    // Parse markdown
    let parsed_doc = quillmark_core::decompose(markdown)
        .map_err(|e| RenderError::Other(format!("Failed to parse markdown: {}", e).into()))?;

    // Create Glue and register filters
    let mut glue = Glue::new(quill.template_content.clone());
    config.backend.register_filters(&mut glue);

    // Render template
    let glue_content = glue.compose(parsed_doc.fields().clone())
        .map_err(|e| RenderError::Other(Box::new(e)))?;

    // Compile
    config.backend.compile(&glue_content, &quill, &engine_config)
}

/// Legacy render configuration (deprecated)
/// 
/// Use `QuillEngine::new()` instead of this configuration struct.
#[deprecated(since = "0.2.0", note = "Use QuillEngine::new() instead")]
pub struct LegacyRenderConfig {
    pub backend: Box<dyn Backend>,
    pub output_format: Option<OutputFormat>,
    pub quill_path: PathBuf,
}

// Keep the original RenderConfig name for backward compatibility  
#[deprecated(since = "0.2.0", note = "Use QuillEngine::new() instead")]
pub type RenderConfig = LegacyRenderConfig;

/// Load quill data from a path
/// 
/// This function loads a quill template from the filesystem using the new standardized
/// approach that prioritizes the quill.toml metadata and supports configurable template files.
fn load_quill_from_path<P: AsRef<Path>>(path: P, _glue_type: &str) -> Result<Quill, RenderError> {
    let path = path.as_ref();
    
    // Use Quill::from_path which handles all the logic including quill.toml parsing
    Quill::from_path(path)
        .map_err(|e| RenderError::Other(e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Pdf, OutputFormat::Pdf);
        assert_ne!(OutputFormat::Pdf, OutputFormat::Svg);
    }

    // Mock backend for testing
    struct MockBackend;

    impl Backend for MockBackend {
        fn id(&self) -> &'static str {
            "mock"
        }

        fn supported_formats(&self) -> &'static [OutputFormat] {
            &[OutputFormat::Txt, OutputFormat::Pdf]
        }

        fn glue_type(&self) -> &'static str {
            ".txt"
        }

        fn register_filters(&self, _glue: &mut Glue) {
            // Mock doesn't register any filters
        }

        fn compile(&self, content: &str, _quill: &Quill, opts: &RenderOptions) -> Result<Vec<Artifact>, RenderError> {
            let format = opts.output_format.unwrap_or(OutputFormat::Txt);
            
            if !self.supported_formats().contains(&format) {
                return Err(RenderError::FormatNotSupported {
                    backend: self.id().to_string(),
                    format,
                });
            }

            Ok(vec![Artifact {
                bytes: format!("Mock output for format {:?}: {}", format, content).into_bytes(),
                output_format: format,
            }])
        }
    }

    #[test]
    fn test_quill_engine_basic_functionality() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create temporary directory for test quill
        let temp_dir = TempDir::new()?;
        let quill_path = temp_dir.path().join("test-quill");
        fs::create_dir_all(&quill_path)?;
        
        // Create quill.toml to specify the correct glue file for MockBackend
        fs::write(
            quill_path.join("quill.toml"),
            r#"[Quill]
name = "test-quill"
version = "1.0.0"
glue_file = "glue.txt"
"#
        )?;
        
        // Create minimal quill template with correct extension for MockBackend
        fs::write(
            quill_path.join("glue.txt"),
            "Test Template: {{ title }} - {{ body }}"
        )?;

        // Create QuillEngine
        let backend = Box::new(MockBackend);
        let engine = QuillEngine::new(backend, quill_path)?;

        // Test basic properties
        assert_eq!(engine.backend_id(), "mock");
        assert_eq!(engine.quill_name(), "test-quill");
        assert_eq!(engine.glue_type(), ".txt");
        assert_eq!(engine.supported_formats(), &[OutputFormat::Txt, OutputFormat::Pdf]);

        // Test rendering
        let markdown = r#"---
title: "Test Document"
---

# Hello World

This is a test document."#;

        let artifacts = engine.render(markdown)?;
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].output_format, OutputFormat::Txt);

        // Check that the output contains template processing
        let output = String::from_utf8(artifacts[0].bytes.clone())?;
        assert!(output.contains("Mock output"));

        Ok(())
    }

    #[test]
    fn test_quill_engine_format_validation() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create temporary directory for test quill
        let temp_dir = TempDir::new()?;
        let quill_path = temp_dir.path().join("test-quill");
        fs::create_dir_all(&quill_path)?;
        
        // Create quill.toml to specify the correct glue file for MockBackend
        fs::write(
            quill_path.join("quill.toml"),
            r#"[Quill]
name = "test-quill"  
glue_file = "glue.txt"
"#
        )?;
        
        // Create minimal quill template with correct extension for MockBackend
        fs::write(
            quill_path.join("glue.txt"),
            "Test Template: {{ body }}"
        )?;

        // Create QuillEngine
        let backend = Box::new(MockBackend);
        let engine = QuillEngine::new(backend, quill_path)?;

        let markdown = "# Test";

        // Test valid format
        let artifacts = engine.render_with_format(markdown, Some(OutputFormat::Txt))?;
        assert_eq!(artifacts[0].output_format, OutputFormat::Txt);

        // Test invalid format
        let result = engine.render_with_format(markdown, Some(OutputFormat::Svg));
        assert!(result.is_err());
        
        match result.unwrap_err() {
            RenderError::FormatNotSupported { backend, format } => {
                assert_eq!(backend, "mock");
                assert_eq!(format, OutputFormat::Svg);
            }
            _ => panic!("Expected FormatNotSupported error"),
        }

        Ok(())
    }
}
