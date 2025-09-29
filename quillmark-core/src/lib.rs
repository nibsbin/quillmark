use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::PathBuf;

// Re-export parsing functionality
pub mod parse;
pub use parse::{decompose, ParsedDocument, BODY_FIELD};

// Re-export templating functionality
pub mod templating;
pub use templating::{Glue, TemplateError};

// Re-export backend trait
pub mod backend;
pub use backend::Backend;

// Re-export error types
pub mod error;
pub use error::{RenderError, RenderResult, Diagnostic, Severity, Location};

/// Output formats supported by backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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

/// Internal rendering options used by engine orchestration
#[derive(Debug)]
pub struct RenderOptions {
    pub output_format: Option<OutputFormat>,
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
    /// Create a Quill from a directory path
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        use std::fs;
        
        let path = path.as_ref();
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();

        // Read quill.toml
        let quill_toml_path = path.join("quill.toml");
        let quill_toml_content = fs::read_to_string(&quill_toml_path)
            .map_err(|e| format!("Failed to read quill.toml: {}", e))?;

        let quill_toml: toml::Value = toml::from_str(&quill_toml_content)
            .map_err(|e| format!("Failed to parse quill.toml: {}", e))?;

        let mut metadata = HashMap::new();
        let mut glue_file = "glue.typ".to_string(); // default

        // Extract metadata from [Quill] section
        if let Some(quill_section) = quill_toml.get("Quill").or_else(|| quill_toml.get("quill")) {
            // Extract glue_file if present
            if let Some(gf) = quill_section.get("glue_file").and_then(|v| v.as_str()) {
                glue_file = gf.to_string();
            }
            
            // Add all fields from quill section to metadata
            if let toml::Value::Table(table) = quill_section {
                for (key, value) in table {
                    if key != "name" && key != "glue_file" { // These are handled specially
                        match Self::toml_to_yaml_value(value) {
                            Ok(yaml_value) => {
                                metadata.insert(key.clone(), yaml_value);
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to convert field '{}': {}", key, e);
                            }
                        }
                    }
                }
            }
        }

        // Read the template content from glue file
        let glue_path = path.join(&glue_file);
        let template_content = fs::read_to_string(&glue_path)
            .map_err(|e| format!("Failed to read glue file '{}': {}", glue_file, e))?;

        Ok(Quill {
            template_content,
            metadata,
            base_path: path.to_path_buf(),
            name,
            glue_file,
        })
    }

    /// Convert TOML value to YAML value
    pub fn toml_to_yaml_value(toml_val: &toml::Value) -> Result<serde_yaml::Value, Box<dyn StdError + Send + Sync>> {
        let json_val = serde_json::to_value(toml_val)?;
        let yaml_val = serde_yaml::to_value(json_val)?;
        Ok(yaml_val)
    }

    /// Get the path to the assets directory
    pub fn assets_path(&self) -> PathBuf {
        self.base_path.join("assets")
    }

    /// Get the path to the packages directory
    pub fn packages_path(&self) -> PathBuf {
        self.base_path.join("packages")
    }

    /// Get the path to the glue file
    pub fn glue_path(&self) -> PathBuf {
        self.base_path.join(&self.glue_file)
    }

    /// Validate the quill structure
    pub fn validate(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        // Check that glue file exists
        if !self.glue_path().exists() {
            return Err(format!("Glue file '{}' does not exist", self.glue_file).into());
        }
        Ok(())
    }
}