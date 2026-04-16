//! Quill loading and construction routines.
use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::{Component, Path};

use crate::value::QuillValue;

use super::{FileTreeNode, Quill, QuillConfig, QuillIgnore};

impl Quill {
    /// Create a Quill from a directory path
    pub fn from_path<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        use std::fs;

        let path = path.as_ref();

        // Load .quillignore if it exists
        let quillignore_path = path.join(".quillignore");
        let ignore = if quillignore_path.exists() {
            let ignore_content = fs::read_to_string(&quillignore_path)
                .map_err(|e| format!("Failed to read .quillignore: {}", e))?;
            QuillIgnore::from_content(&ignore_content)
        } else {
            // Default ignore patterns
            QuillIgnore::new(vec![
                ".git/".to_string(),
                ".gitignore".to_string(),
                ".quillignore".to_string(),
                "target/".to_string(),
                "node_modules/".to_string(),
            ])
        };

        // Load all files into a tree structure
        let root = Self::load_directory_as_tree(path, path, &ignore)?;

        // Create Quill from the file tree
        Self::from_tree(root)
    }

    /// Create a Quill from a tree structure
    ///
    /// This is the authoritative method for creating a Quill from an in-memory file tree.
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

        // Construct Quill from QuillConfig
        Self::from_config(config, root)
    }

    /// Create a Quill from a tree, rehydrating fonts via `provider`.
    ///
    /// If `fonts.json` is present at the root of `root`, the provider is called
    /// for each unique hash listed in the manifest and the bytes are written back
    /// to their original paths before loading proceeds.  If `fonts.json` is
    /// absent (local dev trees, pre-centralization bundles) the provider is
    /// never called and this method is identical to [`from_tree`].
    pub fn from_tree_with_fonts(
        mut root: FileTreeNode,
        provider: &dyn crate::fonts::FontProvider,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        crate::fonts::rehydrate_tree(&mut root, provider)?;
        Self::from_tree(root)
    }

    /// Create a Quill from a QuillConfig and file tree
    ///
    /// This method constructs a Quill from a parsed QuillConfig and validates
    /// all file references.
    ///
    /// # Arguments
    ///
    /// * `config` - The parsed QuillConfig
    ///   (mutable because resolved `example_markdown` content is attached during load)
    /// * `root` - The root node of the file tree
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The plate file specified in config is not found or not valid UTF-8
    /// - The example file specified in config is not found or not valid UTF-8
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
            QuillValue::from_json(serde_json::Value::String(
                config.main().description.clone().unwrap_or_default(),
            )),
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

        // Add typst config to metadata with typst_ prefix
        for (key, value) in &config.typst_config {
            metadata.insert(format!("typst_{}", key), value.clone());
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

        // Extract and cache defaults and examples from config directly
        let defaults = config.defaults();
        let examples = config.examples();

        let quill = Quill {
            metadata,
            name: config.name.clone(),
            backend: config.backend.clone(),
            plate: plate_content,
            example: example_content,
            config,
            defaults,
            examples,
            files: root,
        };

        Ok(quill)
    }

    /// Create a Quill from a JSON representation
    ///
    /// Parses a JSON string into an in-memory file tree and validates it. The
    /// precise JSON contract is documented in `designs/QUILL.md`.
    /// The JSON format MUST have a root object with a `files` key. The optional
    /// `metadata` key provides additional metadata that overrides defaults.
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let root = Self::parse_json_to_tree(json_str)?;
        Self::from_tree(root)
    }

    /// Create a Quill from a JSON representation, rehydrating fonts via `provider`.
    ///
    /// Identical to [`from_json`] except that if `fonts.json` is present in the
    /// tree (i.e. the bundle was published in dehydrated form), font bytes are
    /// fetched from `provider` and written back to their original paths before
    /// loading proceeds.
    ///
    /// This is the entry point used by the WASM binding when Node supplies a
    /// pre-fetched `Map<string, Uint8Array>` alongside the Quill JSON.
    pub fn from_json_with_fonts(
        json_str: &str,
        provider: &dyn crate::fonts::FontProvider,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let root = Self::parse_json_to_tree(json_str)?;
        Self::from_tree_with_fonts(root, provider)
    }

    /// Shared JSON-to-tree parsing used by both `from_json` variants.
    fn parse_json_to_tree(json_str: &str) -> Result<FileTreeNode, Box<dyn StdError + Send + Sync>> {
        use serde_json::Value as JsonValue;

        let json: JsonValue =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let obj = json.as_object().ok_or("Root must be an object")?;

        // Extract files (required)
        let files_obj = obj
            .get("files")
            .and_then(|v| v.as_object())
            .ok_or("Missing or invalid 'files' key")?;

        // Parse file tree
        let mut root_files = HashMap::new();
        for (key, value) in files_obj {
            root_files.insert(key.clone(), FileTreeNode::from_json_value(value)?);
        }

        Ok(FileTreeNode::Directory { files: root_files })
    }

    /// Recursively load all files from a directory into a tree structure
    fn load_directory_as_tree(
        current_dir: &Path,
        base_dir: &Path,
        ignore: &QuillIgnore,
    ) -> Result<FileTreeNode, Box<dyn StdError + Send + Sync>> {
        use std::fs;

        if !current_dir.exists() {
            return Ok(FileTreeNode::Directory {
                files: HashMap::new(),
            });
        }

        let mut files = HashMap::new();

        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_dir)
                .map_err(|e| format!("Failed to get relative path: {}", e))?
                .to_path_buf();

            // Check if this path should be ignored
            if ignore.is_ignored(&relative_path) {
                continue;
            }

            // Get the filename
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| format!("Invalid filename: {}", path.display()))?
                .to_string();

            if path.is_file() {
                let contents = fs::read(&path)
                    .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

                files.insert(filename, FileTreeNode::File { contents });
            } else if path.is_dir() {
                // Recursively process subdirectory
                let subdir_tree = Self::load_directory_as_tree(&path, base_dir, ignore)?;
                files.insert(filename, subdir_tree);
            }
        }

        Ok(FileTreeNode::Directory { files })
    }
}
