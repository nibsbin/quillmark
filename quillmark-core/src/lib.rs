use std::error::Error;
use std::path::PathBuf;
use std::collections::HashMap;

// Re-export parsing functionality
pub mod parse;
pub use parse::{decompose, ParsedDocument, BODY_FIELD};

// Re-export templating functionality
pub mod templating;
pub use templating::{Glue, TemplateError};

/// Output formats supported by backends
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Txt,
    Svg,
    Pdf,
}

/// An artifact produced by rendering
#[derive(Debug)]
pub struct Artifact {
    pub bytes: Vec<u8>,
    pub output_format: OutputFormat,
}

/// Rendering options
pub struct RenderConfig {
    /// The backend to use for rendering. Backends implement the `Backend` trait.
    pub backend: Box<dyn Backend>,
    pub output_format: Option<OutputFormat>,
    pub quill_path: PathBuf, // Path to quill template to use
}

/// Result type for rendering operations
pub type RenderResult = Result<Vec<Artifact>, RenderError>;

/// Errors that can occur during rendering
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("{0:?} backend is not built in this binary")]
    UnsupportedBackend(String),
    #[error("{format:?} not supported by {backend:?}")]
    FormatNotSupported {
        backend: String,
        format: OutputFormat,
    },
    #[error("multiple backends can produce {0:?}; specify one explicitly")]
    AmbiguousBackend(OutputFormat),
    #[error(transparent)]
    Other(#[from] Box<dyn Error + Send + Sync>),
}

/// A quill template containing the template content and metadata with file management capabilities
#[derive(Debug, Clone)]
pub struct Quill {
    /// The template content 
    pub template_content: String,
    /// Quill-specific data that backends might need
    pub metadata: HashMap<String, serde_yaml::Value>,
    /// Base path for resolving relative paths
    pub base_path: PathBuf,
    /// Name of the quill (derived from directory name)
    pub name: String,
    /// Glue template file name
    pub glue_file: String,
}

impl Quill {
    /// Create new Quill with just template content
    pub fn new(template_content: String, base_path: PathBuf) -> Self {
        let name = base_path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("unknown"))
            .to_string_lossy()
            .to_string();
        
        Self {
            template_content,
            metadata: HashMap::new(),
            base_path,
            name,
            glue_file: "glue.typ".to_string(),
        }
    }
    
    /// Create Quill with metadata
    pub fn with_metadata(template_content: String, base_path: PathBuf, metadata: HashMap<String, serde_yaml::Value>) -> Self {
        let name = base_path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("unknown"))
            .to_string_lossy()
            .to_string();
        
        let glue_file = metadata.get("glue_file")
            .and_then(|v| v.as_str())
            .unwrap_or("glue.typ")
            .to_string();
            
        Self {
            template_content,
            metadata,
            base_path,
            name,
            glue_file,
        }
    }
    
    /// Create a Quill from a directory path
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let path = path.as_ref().to_path_buf();
        
        if !path.exists() {
            return Err(format!("Quill path does not exist: {}", path.display()).into());
        }
        
        let name = path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("unknown"))
            .to_string_lossy()
            .to_string();
        
        // Look for glue.typ file (default glue template file)
        let glue_path = path.join("glue.typ");
        if !glue_path.exists() {
            return Err(format!("Glue template file not found: {}", glue_path.display()).into());
        }
        
        let template_content = std::fs::read_to_string(&glue_path)
            .map_err(|e| format!("Failed to read glue template file: {}", e))?;
        
        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), serde_yaml::Value::String(name.clone()));
        metadata.insert("glue_file".to_string(), serde_yaml::Value::String("glue.typ".to_string()));
        
        Ok(Self {
            template_content,
            metadata,
            base_path: path,
            name,
            glue_file: "glue.typ".to_string(),
        })
    }
    
    /// Get the glue template file path
    pub fn glue_path(&self) -> PathBuf {
        self.base_path.join(&self.glue_file)
    }
    
    /// Get the assets directory path
    pub fn assets_path(&self) -> PathBuf {
        self.base_path.join("assets")
    }
    
    /// Get the packages directory path
    pub fn packages_path(&self) -> PathBuf {
        self.base_path.join("packages")
    }
    
    /// Validate that the quill has all necessary components
    pub fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.glue_path().exists() {
            return Err(format!("Glue template file does not exist: {}", self.glue_path().display()).into());
        }
        
        // Assets and packages directories are optional
        Ok(())
    }
}

/// Trait for markdown rendering backends
pub trait Backend: Send + Sync {
    /// Stable identifier (e.g., "typst", "latex", "mock")
    fn id(&self) -> &'static str;

    /// Formats this backend supports in *this* build.
    fn supported_formats(&self) -> &'static [OutputFormat];

    /// File extension for the document type this backend processes (e.g., ".typ", ".tex")
    fn glue_type(&self) -> &'static str;

    /// Register filters with the given Glue instance
    fn register_filters(&self, glue: &mut Glue);

    /// Compile the rendered glue content into final artifacts
    fn compile(&self, glue_content: &str, quill: &Quill, opts: &RenderConfig) -> Result<Vec<Artifact>, RenderError>;
}



/// Test context helpers for examples and testing
pub mod test_context {
    use super::*;
    
    /// Find the workspace root examples directory
    /// This helper searches for the examples/ folder starting from the current directory
    /// and walking up the directory tree until it finds a Cargo.toml at workspace level.
    /// 
    /// Note: This function is deprecated. Use quillmark_fixtures::resource_path("") instead.
    pub fn examples_dir() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        // Try to delegate to quillmark-fixtures if available (in dev builds)
        #[cfg(test)]
        {
            if let Ok(path) = quillmark_fixtures::resource_path("") {
                return Ok(path);
            }
        }
        
        // Fallback to original implementation
        let current_dir = std::env::current_dir()?;
        let mut dir = current_dir.as_path();
        
        // Walk up the directory tree to find workspace root
        loop {
            let cargo_toml = dir.join("Cargo.toml");
            let examples_dir = dir.join("examples");
            
            // Check if this looks like the workspace root (has both Cargo.toml and examples/)
            if cargo_toml.exists() && examples_dir.exists() {
                // Also check if Cargo.toml contains workspace members to confirm it's the workspace root
                if let Ok(cargo_content) = std::fs::read_to_string(&cargo_toml) {
                    if cargo_content.contains("[workspace]") || cargo_content.contains("members") {
                        return Ok(examples_dir);
                    }
                }
                // Fallback: if we have examples/ directory, use it
                return Ok(examples_dir);
            }
            
            // Move up one directory
            if let Some(parent) = dir.parent() {
                dir = parent;
            } else {
                break;
            }
        }
        
        // If we can't find it, create examples/ in current directory
        let fallback_examples = current_dir.join("examples");
        std::fs::create_dir_all(&fallback_examples)?;
        Ok(fallback_examples)
    }
    
    /// Create an output directory within the examples folder
    /// This ensures all example outputs are staged within the workspace examples folder
    /// 
    /// Note: This function is deprecated. Use quillmark_fixtures::example_output_dir(subdir) instead.
    pub fn create_output_dir(subdir: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        // Try to delegate to quillmark-fixtures if available (in dev builds)
        #[cfg(test)]
        {
            if let Ok(path) = quillmark_fixtures::example_output_dir(subdir) {
                return Ok(path);
            }
        }
        
        // Fallback to original implementation
        let examples_root = examples_dir()?;
        let output_dir = examples_root.join(subdir);
        std::fs::create_dir_all(&output_dir)?;
        Ok(output_dir)
    }
    
    /// Get a path to a file within the examples directory
    /// 
    /// Note: This function is deprecated. Use quillmark_fixtures::resource_path(relative_path) instead.
    pub fn examples_path(relative_path: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        // Try to delegate to quillmark-fixtures if available (in dev builds)
        #[cfg(test)]
        {
            if let Ok(path) = quillmark_fixtures::resource_path(relative_path) {
                return Ok(path);
            }
        }
        
        // Fallback to original implementation
        let examples_root = examples_dir()?;
        Ok(examples_root.join(relative_path))
    }
}
