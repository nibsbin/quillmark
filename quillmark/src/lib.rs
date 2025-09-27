// Re-export all types from quillmark-core for backward compatibility
pub use quillmark_core::{Artifact, Backend, Options, OutputFormat, RenderError, RenderResult, QuillData, Glue};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Registry to hold registered backends
static mut BACKEND_REGISTRY: Option<HashMap<String, Box<dyn Backend>>> = None;
static REGISTRY_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize the backend registry
fn init_registry() {
    REGISTRY_INIT.call_once(|| {
        unsafe {
            BACKEND_REGISTRY = Some(HashMap::new());
        }
    });
}

/// Register a backend with the rendering system
pub fn register_backend(backend: Box<dyn Backend>) {
    init_registry();
    unsafe {
        if let Some(ref mut registry) = BACKEND_REGISTRY {
            registry.insert(backend.id().to_string(), backend);
        }
    }
}

/// Get all registered backends
#[allow(static_mut_refs)]
fn get_backends() -> Option<&'static HashMap<String, Box<dyn Backend>>> {
    unsafe { BACKEND_REGISTRY.as_ref() }
}

/// Render markdown using the specified options
///
/// This function orchestrates the rendering process:
/// 1. Selects appropriate backend
/// 2. Loads quill template if specified
/// 3. Parses markdown and extracts frontmatter
/// 4. Creates template glue and registers backend filters
/// 5. Renders template to produce glue content
/// 6. Calls backend to compile final artifacts
pub fn render(markdown: &str, options: &Options) -> RenderResult {
    // Select backend
    let backend = select_backend(options)?;
    
    // Load quill data
    let quill_data = load_quill_data(options, backend.glue_type())?;
    
    // Parse markdown to extract frontmatter and body
    let parsed_doc = quillmark_core::decompose(markdown)
        .map_err(|e| RenderError::Other(format!("Failed to parse markdown: {}", e).into()))?;
    
    // Create Glue instance with the template and register backend filters
    let mut glue = Glue::new(quill_data.template_content.clone());
    
    // Register filters from the backend
    backend.register_filters(&mut glue);
    
    // Render the template with the parsed document context
    let glue_content = glue.compose(parsed_doc.fields().clone())
        .map_err(|e| RenderError::Other(Box::new(e)))?;
    
    // Call the backend to compile the final artifacts
    backend.compile(&glue_content, &quill_data, options)
}

/// Select the appropriate backend based on options
fn select_backend(options: &Options) -> Result<&'static Box<dyn Backend>, RenderError> {
    let backends = get_backends().ok_or_else(|| {
        RenderError::UnsupportedBackend("no backends registered".to_string())
    })?;

    if let Some(backend_id) = &options.backend {
        backends.get(backend_id).ok_or_else(|| {
            RenderError::UnsupportedBackend(backend_id.clone())
        })
    } else if let Some(format) = options.format {
        // Find backend that supports the requested format
        let supporting_backends: Vec<_> = backends
            .values()
            .filter(|b| b.supported_formats().contains(&format))
            .collect();

        match supporting_backends.len() {
            0 => Err(RenderError::FormatNotSupported {
                backend: "any".to_string(),
                format,
            }),
            1 => Ok(supporting_backends[0]),
            _ => Err(RenderError::AmbiguousBackend(format)),
        }
    } else {
        Err(RenderError::UnsupportedBackend(
            "no backend or format specified".to_string(),
        ))
    }
}

/// Load quill template data
fn load_quill_data(options: &Options, glue_type: &str) -> Result<QuillData, RenderError> {
    if let Some(quill_path) = &options.quill_path {
        // Load from specified path
        load_quill_from_path(quill_path, glue_type)
    } else {
        // For now, require explicit quill path
        // In future, could support default quill discovery
        Err(RenderError::Other(
            "No quill template specified. Use Options::quill_path".into(),
        ))
    }
}

/// Load quill data from a path
fn load_quill_from_path<P: AsRef<Path>>(path: P, glue_type: &str) -> Result<QuillData, RenderError> {
    let path = path.as_ref();
    
    // Check if path exists
    if !path.exists() {
        return Err(RenderError::Other(
            format!("Quill path does not exist: {}", path.display()).into(),
        ));
    }

    // Determine main template file - look for "glue" + glue_type
    let main_file_name = format!("glue{}", glue_type);
    let main_file_path = path.join(&main_file_name);
    
    if !main_file_path.exists() {
        return Err(RenderError::Other(
            format!("Main template file not found: {}", main_file_path.display()).into(),
        ));
    }

    // Read template content
    let template_content = fs::read_to_string(&main_file_path)
        .map_err(|e| RenderError::Other(format!("Failed to read template file: {}", e).into()))?;

    // Create quill data
    let mut metadata = HashMap::new();
    
    // Add basic metadata
    metadata.insert(
        "name".to_string(),
        serde_yaml::Value::String(
            path.file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("unknown"))
                .to_string_lossy()
                .to_string()
        ),
    );
    
    metadata.insert(
        "main_file".to_string(),
        serde_yaml::Value::String(main_file_name),
    );

    Ok(QuillData::with_metadata(
        template_content,
        path.to_path_buf(),
        metadata,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_with_no_backends() {
        let options = Options {
            backend: None,
            format: Some(OutputFormat::Pdf),
            quill_path: None,
        };

        let result = render("# Hello World", &options);
        assert!(result.is_err());

        match result.unwrap_err() {
            RenderError::UnsupportedBackend(_) => {}
            _ => panic!("Expected UnsupportedBackend error"),
        }
    }

    #[test]
    fn test_render_with_no_quill_path() {
        // This will need a registered backend to proceed far enough to get the quill error
        register_backend(Box::new(TestBackend));
        
        let options = Options {
            backend: Some("test".to_string()),
            format: Some(OutputFormat::Pdf),
            quill_path: None,
        };

        let result = render("# Hello World", &options);
        assert!(result.is_err());

        // Should fail because no quill path specified
        match result.unwrap_err() {
            RenderError::Other(_) => {}
            other => panic!("Expected Other error, got: {:?}", other),
        }
    }

    #[test]
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Pdf, OutputFormat::Pdf);
        assert_ne!(OutputFormat::Pdf, OutputFormat::Svg);
    }

    // Simple test backend
    struct TestBackend;
    
    impl Backend for TestBackend {
        fn id(&self) -> &'static str { "test" }
        fn supported_formats(&self) -> &'static [OutputFormat] { &[OutputFormat::Pdf] }
        fn glue_type(&self) -> &'static str { ".test" }
        fn register_filters(&self, _glue: &mut Glue) {}
        fn compile(&self, _glue_content: &str, _quill_data: &QuillData, _opts: &Options) -> Result<Vec<Artifact>, RenderError> {
            Ok(vec![])
        }
    }
}
