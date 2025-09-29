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
        
        // Default name from directory
        let default_name = path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("unknown"))
            .to_string_lossy()
            .to_string();
        
        // Try to load quill.toml first
        let mut metadata = HashMap::new();
        let mut name = default_name.clone();
        let mut glue_file = "glue.typ".to_string();
        
        let toml_path = path.join("quill.toml");
        if toml_path.exists() {
            let toml_content = std::fs::read_to_string(&toml_path)
                .map_err(|e| format!("Failed to read quill.toml: {}", e))?;
            
            let toml_value: toml::Value = toml::from_str(&toml_content)
                .map_err(|e| format!("Failed to parse quill.toml: {}", e))?;
            
            if let Some(quill_section) = toml_value.get("Quill") {
                // Extract name if present
                if let Some(toml_name) = quill_section.get("name").and_then(|v| v.as_str()) {
                    name = toml_name.to_string();
                }
                
                // Extract version if present
                if let Some(version) = quill_section.get("version").and_then(|v| v.as_str()) {
                    metadata.insert("version".to_string(), serde_yaml::Value::String(version.to_string()));
                }
                
                // Extract glue_file if present
                if let Some(gf) = quill_section.get("glue_file").and_then(|v| v.as_str()) {
                    glue_file = gf.to_string();
                }
                
                // Add all other fields from quill section to metadata
                if let toml::Value::Table(table) = quill_section {
                    for (key, value) in table {
                        if key != "name" && key != "glue_file" { // These are handled specially
                            match Self::toml_to_yaml_value(value) {
                                Ok(yaml_value) => {
                                    metadata.insert(key.clone(), yaml_value);
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to convert TOML value for key '{}': {}", key, e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Always add name and glue_file to metadata
        metadata.insert("name".to_string(), serde_yaml::Value::String(name.clone()));
        metadata.insert("glue_file".to_string(), serde_yaml::Value::String(glue_file.clone()));
        
        // Look for glue template file
        let glue_path = path.join(&glue_file);
        if !glue_path.exists() {
            return Err(format!("Glue template file not found: {}", glue_path.display()).into());
        }
        
        let template_content = std::fs::read_to_string(&glue_path)
            .map_err(|e| format!("Failed to read glue template file: {}", e))?;
        
        Ok(Self {
            template_content,
            metadata,
            base_path: path,
            name,
            glue_file,
        })
    }
    
    /// Convert TOML value to YAML value for metadata storage
    fn toml_to_yaml_value(toml_val: &toml::Value) -> Result<serde_yaml::Value, Box<dyn Error + Send + Sync>> {
        match toml_val {
            toml::Value::String(s) => Ok(serde_yaml::Value::String(s.clone())),
            toml::Value::Integer(i) => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*i))),
            toml::Value::Float(f) => Ok(serde_yaml::Value::Number(serde_yaml::Number::from(*f))),
            toml::Value::Boolean(b) => Ok(serde_yaml::Value::Bool(*b)),
            toml::Value::Datetime(dt) => Ok(serde_yaml::Value::String(dt.to_string())),
            toml::Value::Array(arr) => {
                let mut yaml_array = Vec::new();
                for item in arr {
                    yaml_array.push(Self::toml_to_yaml_value(item)?);
                }
                Ok(serde_yaml::Value::Sequence(yaml_array))
            }
            toml::Value::Table(table) => {
                let mut yaml_map = serde_yaml::Mapping::new();
                for (key, value) in table {
                    yaml_map.insert(
                        serde_yaml::Value::String(key.clone()),
                        Self::toml_to_yaml_value(value)?
                    );
                }
                Ok(serde_yaml::Value::Mapping(yaml_map))
            }
        }
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
    pub fn examples_dir() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
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
    pub fn create_output_dir(subdir: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let examples_root = examples_dir()?;
        let output_dir = examples_root.join(subdir);
        std::fs::create_dir_all(&output_dir)?;
        Ok(output_dir)
    }
    
    /// Get a path to a file within the examples directory
    pub fn examples_path(relative_path: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let examples_root = examples_dir()?;
        Ok(examples_root.join(relative_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_quill_with_toml_metadata() -> Result<(), Box<dyn Error + Send + Sync>> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let quill_path = temp_dir.path().join("test-quill");
        fs::create_dir_all(&quill_path).map_err(|e| e.to_string())?;

        // Create quill.toml
        let toml_content = r#"
[Quill]
name = "my-test-quill"
version = "0.2.0"
author = "Test Author"
description = "A test quill for validation"
"#;
        fs::write(quill_path.join("quill.toml"), toml_content).map_err(|e| e.to_string())?;

        // Create glue.typ
        let glue_content = r#"= Test Document
This is a test: $content$"#;
        fs::write(quill_path.join("glue.typ"), glue_content).map_err(|e| e.to_string())?;

        // Load the quill
        let quill = Quill::from_path(&quill_path)?;
        
        assert_eq!(quill.name, "my-test-quill");
        assert_eq!(quill.glue_file, "glue.typ");
        
        // Check metadata
        assert_eq!(quill.metadata.get("version").and_then(|v| v.as_str()), Some("0.2.0"));
        assert_eq!(quill.metadata.get("author").and_then(|v| v.as_str()), Some("Test Author"));
        assert_eq!(quill.metadata.get("description").and_then(|v| v.as_str()), Some("A test quill for validation"));
        assert_eq!(quill.metadata.get("name").and_then(|v| v.as_str()), Some("my-test-quill"));

        quill.validate()?;
        Ok(())
    }

    #[test]
    fn test_quill_without_toml_metadata() -> Result<(), Box<dyn Error + Send + Sync>> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let quill_path = temp_dir.path().join("test-quill");
        fs::create_dir_all(&quill_path).map_err(|e| e.to_string())?;

        // Create glue.typ only (no quill.toml)
        let glue_content = r#"= Test Document
This is a test: $content$"#;
        fs::write(quill_path.join("glue.typ"), glue_content).map_err(|e| e.to_string())?;

        // Load the quill
        let quill = Quill::from_path(&quill_path)?;
        
        // Should use directory name as default
        assert_eq!(quill.name, "test-quill");
        assert_eq!(quill.glue_file, "glue.typ");
        
        // Should have basic metadata
        assert_eq!(quill.metadata.get("name").and_then(|v| v.as_str()), Some("test-quill"));
        assert_eq!(quill.metadata.get("glue_file").and_then(|v| v.as_str()), Some("glue.typ"));
        assert!(quill.metadata.get("version").is_none());

        quill.validate()?;
        Ok(())
    }

    #[test]
    fn test_quill_custom_glue_file() -> Result<(), Box<dyn Error + Send + Sync>> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let quill_path = temp_dir.path().join("test-quill");
        fs::create_dir_all(&quill_path).map_err(|e| e.to_string())?;

        // Create quill.toml with custom glue file
        let toml_content = r#"
[Quill]
name = "custom-quill"
version = "1.0.0"
glue_file = "custom.typ"
"#;
        fs::write(quill_path.join("quill.toml"), toml_content).map_err(|e| e.to_string())?;

        // Create custom.typ
        let glue_content = r#"= Custom Template
Custom content: $content$"#;
        fs::write(quill_path.join("custom.typ"), glue_content).map_err(|e| e.to_string())?;

        // Load the quill
        let quill = Quill::from_path(&quill_path)?;
        
        assert_eq!(quill.name, "custom-quill");
        assert_eq!(quill.glue_file, "custom.typ");
        assert_eq!(quill.metadata.get("glue_file").and_then(|v| v.as_str()), Some("custom.typ"));

        quill.validate()?;
        Ok(())
    }

    #[test]
    fn test_quill_toml_conversion() -> Result<(), Box<dyn Error + Send + Sync>> {
        let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
        let quill_path = temp_dir.path().join("test-quill");
        fs::create_dir_all(&quill_path).map_err(|e| e.to_string())?;

        // Create quill.toml with various data types
        let toml_content = r#"
[Quill]
name = "type-test-quill"
version = "1.0.0"
port = 8080
debug = true
tags = ["test", "example"]

[Quill.config]
timeout = 30
enabled = false
"#;
        fs::write(quill_path.join("quill.toml"), toml_content).map_err(|e| e.to_string())?;

        // Create glue.typ
        let glue_content = r#"= Type Test
Test content: $content$"#;
        fs::write(quill_path.join("glue.typ"), glue_content).map_err(|e| e.to_string())?;

        // Load the quill
        let quill = Quill::from_path(&quill_path)?;
        
        assert_eq!(quill.name, "type-test-quill");
        
        // Check various data types in metadata
        assert_eq!(quill.metadata.get("port").and_then(|v| v.as_i64()), Some(8080));
        assert_eq!(quill.metadata.get("debug").and_then(|v| v.as_bool()), Some(true));
        
        // Check array
        if let Some(tags) = quill.metadata.get("tags") {
            if let Some(seq) = tags.as_sequence() {
                assert_eq!(seq.len(), 2);
                assert_eq!(seq[0].as_str(), Some("test"));
                assert_eq!(seq[1].as_str(), Some("example"));
            } else {
                panic!("Tags should be a sequence");
            }
        } else {
            panic!("Tags should be present");
        }

        // Check nested config
        if let Some(config) = quill.metadata.get("config") {
            if let Some(mapping) = config.as_mapping() {
                let timeout = mapping.get(&serde_yaml::Value::String("timeout".to_string()))
                    .and_then(|v| v.as_i64());
                assert_eq!(timeout, Some(30));
                
                let enabled = mapping.get(&serde_yaml::Value::String("enabled".to_string()))
                    .and_then(|v| v.as_bool());
                assert_eq!(enabled, Some(false));
            } else {
                panic!("Config should be a mapping");
            }
        } else {
            panic!("Config should be present");
        }

        quill.validate()?;
        Ok(())
    }

    #[test]
    fn test_demo_quill_loading() -> Result<(), Box<dyn Error + Send + Sync>> {
        // Test loading our demo quill if it exists
        let demo_path = std::path::PathBuf::from("/tmp/demo-quill");
        if demo_path.exists() {
            let quill = Quill::from_path(&demo_path)?;
            
            assert_eq!(quill.name, "demo-quill");
            assert_eq!(quill.glue_file, "template.typ");
            assert_eq!(quill.metadata.get("version").and_then(|v| v.as_str()), Some("2.1.0"));
            assert_eq!(quill.metadata.get("author").and_then(|v| v.as_str()), Some("Demo Author"));
            assert_eq!(quill.metadata.get("license").and_then(|v| v.as_str()), Some("MIT"));
            
            // Test array parsing
            if let Some(tags) = quill.metadata.get("tags") {
                if let Some(seq) = tags.as_sequence() {
                    assert_eq!(seq.len(), 3);
                    assert_eq!(seq[0].as_str(), Some("demo"));
                    assert_eq!(seq[1].as_str(), Some("test"));
                    assert_eq!(seq[2].as_str(), Some("example"));
                }
            }
            
            // Test nested config
            if let Some(config) = quill.metadata.get("config") {
                if let Some(mapping) = config.as_mapping() {
                    let debug = mapping.get(&serde_yaml::Value::String("debug".to_string()))
                        .and_then(|v| v.as_bool());
                    assert_eq!(debug, Some(true));
                    
                    let max_pages = mapping.get(&serde_yaml::Value::String("max_pages".to_string()))
                        .and_then(|v| v.as_i64());
                    assert_eq!(max_pages, Some(100));
                    
                    let default_font = mapping.get(&serde_yaml::Value::String("default_font".to_string()))
                        .and_then(|v| v.as_str());
                    assert_eq!(default_font, Some("Times New Roman"));
                }
            }
            
            quill.validate()?;
        }
        Ok(())
    }
}
