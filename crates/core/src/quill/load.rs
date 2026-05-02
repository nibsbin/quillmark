//! QuillSource loading and construction routines.
use std::error::Error as StdError;
use std::path::{Component, Path};

use crate::value::QuillValue;

use super::{FileTreeNode, QuillConfig, QuillSource};

impl QuillSource {
    /// Create a QuillSource from a tree structure.
    ///
    /// This is the authoritative method for creating a QuillSource from an
    /// in-memory file tree. Filesystem walking belongs upstream (see
    /// `quillmark::Quillmark::quill_from_path`).
    ///
    /// # Arguments
    ///
    /// * `root` - The root node of the file tree
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Quill.yaml is not found in the file tree
    /// - Quill.yaml is not valid UTF-8 or YAML
    /// - The plate file specified in Quill.yaml is not found or not valid UTF-8
    /// - Validation fails
    pub fn from_tree(root: FileTreeNode) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        // Read Quill.yaml
        let quill_yaml_bytes = root
            .get_file("Quill.yaml")
            .ok_or("Quill.yaml not found in file tree")?;

        let quill_yaml_content = String::from_utf8(quill_yaml_bytes.to_vec())
            .map_err(|e| format!("Quill.yaml is not valid UTF-8: {}", e))?;

        // Parse YAML into QuillConfig
        let config = QuillConfig::from_yaml(&quill_yaml_content)?;

        // Construct QuillSource from QuillConfig
        Self::from_config(config, root)
    }

    /// Create a QuillSource from a QuillConfig and file tree.
    fn from_config(
        mut config: QuillConfig,
        root: FileTreeNode,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        // Build metadata from config
        let mut metadata = config.metadata.clone();

        // Add backend to metadata
        metadata.insert(
            "backend".to_string(),
            QuillValue::from_json(serde_json::Value::String(config.backend.clone())),
        );

        metadata.insert(
            "description".to_string(),
            QuillValue::from_json(serde_json::Value::String(config.description.clone())),
        );

        // Add author
        metadata.insert(
            "author".to_string(),
            QuillValue::from_json(serde_json::Value::String(config.author.clone())),
        );

        // Add version
        metadata.insert(
            "version".to_string(),
            QuillValue::from_json(serde_json::Value::String(config.version.clone())),
        );

        // Expose backend-specific config to metadata under `<backend>_<key>`.
        for (key, value) in &config.backend_config {
            metadata.insert(format!("{}_{}", config.backend, key), value.clone());
        }

        // Read the plate content from plate file (if specified)
        let plate_content: Option<String> = if let Some(ref plate_file_name) = config.plate_file {
            let plate_bytes = root.get_file(plate_file_name).ok_or_else(|| {
                format!("Plate file '{}' not found in file tree", plate_file_name)
            })?;

            let content = String::from_utf8(plate_bytes.to_vec()).map_err(|e| {
                format!("Plate file '{}' is not valid UTF-8: {}", plate_file_name, e)
            })?;
            Some(content)
        } else {
            // No plate file specified
            None
        };

        // Read the markdown example content if specified, or check for default "example.md"
        let example_content = if let Some(ref example_file_name) = config.example_file {
            let example_path = Path::new(example_file_name);
            if example_path.is_absolute()
                || example_path
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)))
            {
                return Err(format!(
                    "Example file '{}' is outside the quill directory",
                    example_file_name
                )
                .into());
            }

            let bytes = root.get_file(example_file_name).ok_or_else(|| {
                format!(
                    "Example file '{}' referenced in Quill.yaml not found",
                    example_file_name
                )
            })?;
            Some(String::from_utf8(bytes.to_vec()).map_err(|e| {
                format!(
                    "Example file '{}' is not valid UTF-8: {}",
                    example_file_name, e
                )
            })?)
        } else if root.file_exists("example.md") {
            // Smart default: use example.md if it exists
            let bytes = root
                .get_file("example.md")
                .expect("invariant violation: file_exists(example.md) but get_file returned None");
            Some(String::from_utf8(bytes.to_vec()).map_err(|e| {
                format!(
                    "Default example file 'example.md' is not valid UTF-8: {}",
                    e
                )
            })?)
        } else {
            None
        };

        config.example_markdown = example_content.clone();

        let source = QuillSource {
            metadata,
            name: config.name.clone(),
            backend_id: config.backend.clone(),
            plate: plate_content,
            example: example_content,
            config,
            files: root,
        };

        Ok(source)
    }
}
