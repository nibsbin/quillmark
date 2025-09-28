// Re-export all types from quillmark-core for backward compatibility
pub use quillmark_core::{Artifact, Backend, RenderConfig, OutputFormat, RenderError, RenderResult, QuillData, Glue};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Render markdown using the specified options
///
/// This function orchestrates the rendering process:
/// 1. Selects appropriate backend
/// 2. Loads quill template if specified
/// 3. Parses markdown and extracts frontmatter
/// 4. Creates template glue and registers backend filters
/// 5. Renders template to produce glue content
/// 6. Calls backend to compile final artifacts
pub fn render(markdown: &str, config: &RenderConfig) -> RenderResult {
    // Backend is provided directly in RenderConfig
    let backend = &config.backend;
    
    // Load quill data
    let quill_data = load_quill_data(config, backend.glue_type())?;
    
    // Parse markdown to extract frontmatter and body
    let parsed_doc = quillmark_core::decompose(markdown)
        .map_err(|e| RenderError::Other(format!("Failed to parse markdown: {}", e).into()))?;
    
    // Create Glue instance with the template and register backend filters
    let mut glue = Glue::new(quill_data.template_content.clone());
    
    // Register filters from the backend
    backend.register_filters(&mut glue);
    
    // Render the template with the parsed docucomposement context
    let glue_content = glue.compose(parsed_doc.fields().clone())
        .map_err(|e| RenderError::Other(Box::new(e)))?;

    println!("Glue content: {}", glue_content);
    
    // Call the backend to compile the final artifacts
    backend.compile(&glue_content, &quill_data, config)
}

/// Load quill template data
fn load_quill_data(options: &RenderConfig, glue_type: &str) -> Result<QuillData, RenderError> {
    load_quill_from_path(&options.quill_path, glue_type)
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
    let glue_file_name = format!("glue{}", glue_type);
    let glue_file_path = path.join(&glue_file_name);
    
    if !glue_file_path.exists() {
        return Err(RenderError::Other(
            format!("Main template file not found: {}", glue_file_path.display()).into(),
        ));
    }

    // Read template content
    let template_content = fs::read_to_string(&glue_file_path)
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
        "glue_file".to_string(),
        serde_yaml::Value::String(glue_file_name),
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
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Pdf, OutputFormat::Pdf);
        assert_ne!(OutputFormat::Pdf, OutputFormat::Svg);
    }
}
