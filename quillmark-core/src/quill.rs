//! Quill template bundle types and implementations.

use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::{Path, PathBuf};

use crate::value::QuillValue;

/// Schema definition for a template field
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSchema {
    /// Field type hint (e.g., "string", "number", "boolean", "object", "array")
    pub r#type: Option<String>,
    /// Whether the field is required
    pub required: bool,
    /// Description of the field
    pub description: String,
    /// Example value for the field
    pub example: Option<QuillValue>,
    /// Default value for the field
    pub default: Option<QuillValue>,
}

impl FieldSchema {
    /// Create a new FieldSchema with default values
    pub fn new(description: String) -> Self {
        Self {
            r#type: None,
            required: false,
            description,
            example: None,
            default: None,
        }
    }

    /// Parse a FieldSchema from a QuillValue
    pub fn from_quill_value(value: &QuillValue) -> Result<Self, String> {
        let obj = value
            .as_object()
            .ok_or_else(|| "Field schema must be an object".to_string())?;

        let description = obj
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let required = obj
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let field_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let example = obj.get("example").map(|v| QuillValue::from_json(v.clone()));

        let default = obj.get("default").map(|v| QuillValue::from_json(v.clone()));

        Ok(Self {
            r#type: field_type,
            required,
            description,
            example,
            default,
        })
    }

    /// Convert the FieldSchema to a QuillValue for serialization
    pub fn to_quill_value(&self) -> QuillValue {
        let mut map = serde_json::Map::new();

        map.insert(
            "description".to_string(),
            serde_json::Value::String(self.description.clone()),
        );

        map.insert(
            "required".to_string(),
            serde_json::Value::Bool(self.required),
        );

        if let Some(ref field_type) = self.r#type {
            map.insert(
                "type".to_string(),
                serde_json::Value::String(field_type.clone()),
            );
        }

        if let Some(ref example) = self.example {
            map.insert("example".to_string(), example.as_json().clone());
        }

        if let Some(ref default) = self.default {
            map.insert("default".to_string(), default.as_json().clone());
        }

        QuillValue::from_json(serde_json::Value::Object(map))
    }
}

/// A node in the file tree structure
#[derive(Debug, Clone)]
pub enum FileTreeNode {
    /// A file with its contents
    File {
        /// The file contents as bytes or UTF-8 string
        contents: Vec<u8>,
    },
    /// A directory containing other files and directories
    Directory {
        /// The files and subdirectories in this directory
        files: HashMap<String, FileTreeNode>,
    },
}

impl FileTreeNode {
    /// Get a file or directory node by path
    pub fn get_node<P: AsRef<Path>>(&self, path: P) -> Option<&FileTreeNode> {
        let path = path.as_ref();

        // Handle root path
        if path == Path::new("") {
            return Some(self);
        }

        // Split path into components
        let components: Vec<_> = path
            .components()
            .filter_map(|c| {
                if let std::path::Component::Normal(s) = c {
                    s.to_str()
                } else {
                    None
                }
            })
            .collect();

        if components.is_empty() {
            return Some(self);
        }

        // Navigate through the tree
        let mut current_node = self;
        for component in components {
            match current_node {
                FileTreeNode::Directory { files } => {
                    current_node = files.get(component)?;
                }
                FileTreeNode::File { .. } => {
                    return None; // Can't traverse into a file
                }
            }
        }

        Some(current_node)
    }

    /// Get file contents by path
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&[u8]> {
        match self.get_node(path)? {
            FileTreeNode::File { contents } => Some(contents.as_slice()),
            FileTreeNode::Directory { .. } => None,
        }
    }

    /// Check if a file exists at the given path
    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        matches!(self.get_node(path), Some(FileTreeNode::File { .. }))
    }

    /// Check if a directory exists at the given path
    pub fn dir_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        matches!(self.get_node(path), Some(FileTreeNode::Directory { .. }))
    }

    /// List all files in a directory (non-recursive)
    pub fn list_files<P: AsRef<Path>>(&self, dir_path: P) -> Vec<String> {
        match self.get_node(dir_path) {
            Some(FileTreeNode::Directory { files }) => files
                .iter()
                .filter_map(|(name, node)| {
                    if matches!(node, FileTreeNode::File { .. }) {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// List all subdirectories in a directory (non-recursive)
    pub fn list_subdirectories<P: AsRef<Path>>(&self, dir_path: P) -> Vec<String> {
        match self.get_node(dir_path) {
            Some(FileTreeNode::Directory { files }) => files
                .iter()
                .filter_map(|(name, node)| {
                    if matches!(node, FileTreeNode::Directory { .. }) {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Insert a file or directory at the given path
    pub fn insert<P: AsRef<Path>>(
        &mut self,
        path: P,
        node: FileTreeNode,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let path = path.as_ref();

        // Split path into components
        let components: Vec<_> = path
            .components()
            .filter_map(|c| {
                if let std::path::Component::Normal(s) = c {
                    s.to_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();

        if components.is_empty() {
            return Err("Cannot insert at root path".into());
        }

        // Navigate to parent directory, creating directories as needed
        let mut current_node = self;
        for component in &components[..components.len() - 1] {
            match current_node {
                FileTreeNode::Directory { files } => {
                    current_node =
                        files
                            .entry(component.clone())
                            .or_insert_with(|| FileTreeNode::Directory {
                                files: HashMap::new(),
                            });
                }
                FileTreeNode::File { .. } => {
                    return Err("Cannot traverse into a file".into());
                }
            }
        }

        // Insert the new node
        let filename = &components[components.len() - 1];
        match current_node {
            FileTreeNode::Directory { files } => {
                files.insert(filename.clone(), node);
                Ok(())
            }
            FileTreeNode::File { .. } => Err("Cannot insert into a file".into()),
        }
    }

    /// Parse a tree structure from JSON value
    fn from_json_value(value: &serde_json::Value) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        if let Some(contents_str) = value.get("contents").and_then(|v| v.as_str()) {
            // It's a file with string contents
            Ok(FileTreeNode::File {
                contents: contents_str.as_bytes().to_vec(),
            })
        } else if let Some(bytes_array) = value.get("contents").and_then(|v| v.as_array()) {
            // It's a file with byte array contents
            let contents: Vec<u8> = bytes_array
                .iter()
                .filter_map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok()))
                .collect();
            Ok(FileTreeNode::File { contents })
        } else if let Some(obj) = value.as_object() {
            // It's a directory (either empty or with nested files)
            let mut files = HashMap::new();
            for (name, child_value) in obj {
                files.insert(name.clone(), Self::from_json_value(child_value)?);
            }
            // Empty directories are valid
            Ok(FileTreeNode::Directory { files })
        } else {
            Err(format!("Invalid file tree node: {:?}", value).into())
        }
    }
}

/// Simple gitignore-style pattern matcher for .quillignore
#[derive(Debug, Clone)]
pub struct QuillIgnore {
    patterns: Vec<String>,
}

impl QuillIgnore {
    /// Create a new QuillIgnore from pattern strings
    pub fn new(patterns: Vec<String>) -> Self {
        Self { patterns }
    }

    /// Parse .quillignore content into patterns
    pub fn from_content(content: &str) -> Self {
        let patterns = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_string())
            .collect();
        Self::new(patterns)
    }

    /// Check if a path should be ignored
    pub fn is_ignored<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        let path_str = path.to_string_lossy();

        for pattern in &self.patterns {
            if self.matches_pattern(pattern, &path_str) {
                return true;
            }
        }
        false
    }

    /// Simple pattern matching (supports * wildcard and directory patterns)
    fn matches_pattern(&self, pattern: &str, path: &str) -> bool {
        // Handle directory patterns
        if pattern.ends_with('/') {
            let pattern_prefix = &pattern[..pattern.len() - 1];
            return path.starts_with(pattern_prefix)
                && (path.len() == pattern_prefix.len()
                    || path.chars().nth(pattern_prefix.len()) == Some('/'));
        }

        // Handle exact matches
        if !pattern.contains('*') {
            return path == pattern || path.ends_with(&format!("/{}", pattern));
        }

        // Simple wildcard matching
        if pattern == "*" {
            return true;
        }

        // Handle patterns with wildcards
        let pattern_parts: Vec<&str> = pattern.split('*').collect();
        if pattern_parts.len() == 2 {
            let (prefix, suffix) = (pattern_parts[0], pattern_parts[1]);
            if prefix.is_empty() {
                return path.ends_with(suffix);
            } else if suffix.is_empty() {
                return path.starts_with(prefix);
            } else {
                return path.starts_with(prefix) && path.ends_with(suffix);
            }
        }

        false
    }
}

/// A quill template bundle.
#[derive(Debug, Clone)]
pub struct Quill {
    /// The template content
    pub glue_template: String,
    /// Quill-specific metadata
    pub metadata: HashMap<String, QuillValue>,
    /// Name of the quill
    pub name: String,
    /// Backend identifier (e.g., "typst")
    pub backend: String,
    /// Glue template file name (optional)
    pub glue_file: Option<String>,
    /// Markdown template content (optional)
    pub example: Option<String>,
    /// Field schema documentation (optional)
    pub field_schemas: HashMap<String, FieldSchema>,
    /// In-memory file system (tree structure)
    pub files: FileTreeNode,
}

impl Quill {
    /// Create a Quill from a directory path
    pub fn from_path<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        use std::fs;

        let path = path.as_ref();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();

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
        Self::from_tree(root, Some(name))
    }

    /// Create a Quill from a tree structure
    ///
    /// This is the authoritative method for creating a Quill from an in-memory file tree.
    ///
    /// # Arguments
    ///
    /// * `root` - The root node of the file tree
    /// * `default_name` - Optional default name (will be overridden by name in Quill.toml)
    /// * `default_name` - Optional default name (will be overridden by name in Quill.toml)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Quill.toml is not found in the file tree
    /// - Quill.toml is not valid UTF-8 or TOML
    /// - The glue file specified in Quill.toml is not found or not valid UTF-8
    /// - Validation fails
    pub fn from_tree(
        root: FileTreeNode,
        default_name: Option<String>,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        // Read Quill.toml
        let quill_toml_bytes = root
            .get_file("Quill.toml")
            .ok_or("Quill.toml not found in file tree")?;

        let quill_toml_content = String::from_utf8(quill_toml_bytes.to_vec())
            .map_err(|e| format!("Quill.toml is not valid UTF-8: {}", e))?;

        let quill_toml: toml::Value = toml::from_str(&quill_toml_content)
            .map_err(|e| format!("Failed to parse Quill.toml: {}", e))?;

        let mut metadata = HashMap::new();
        let mut glue_file: Option<String> = None;
        let mut template_file: Option<String> = None;
        let mut quill_name = default_name.unwrap_or_else(|| "unnamed".to_string());
        let mut backend = String::new();
        let mut field_schemas = HashMap::new();

        // Extract fields from [Quill] section
        if let Some(quill_section) = quill_toml.get("Quill") {
            // Extract required fields: name, backend, glue, template
            if let Some(name_val) = quill_section.get("name").and_then(|v| v.as_str()) {
                quill_name = name_val.to_string();
            }

            if let Some(backend_val) = quill_section.get("backend").and_then(|v| v.as_str()) {
                backend = backend_val.to_string();
                match QuillValue::from_toml(&toml::Value::String(backend_val.to_string())) {
                    Ok(quill_value) => {
                        metadata.insert("backend".to_string(), quill_value);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to convert backend field: {}", e);
                    }
                }
            }

            if let Some(glue_val) = quill_section.get("glue").and_then(|v| v.as_str()) {
                glue_file = Some(glue_val.to_string());
            }

            if let Some(example_val) = quill_section.get("example").and_then(|v| v.as_str()) {
                template_file = Some(example_val.to_string());
            }

            // Validate that description is present and non-empty (required field)
            let description = quill_section
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or("Missing required 'description' field in [Quill] section")?;

            if description.trim().is_empty() {
                return Err("'description' field in [Quill] section cannot be empty".into());
            }

            // Add other fields to metadata (excluding special fields and version)
            if let toml::Value::Table(table) = quill_section {
                for (key, value) in table {
                    if key != "name"
                        && key != "backend"
                        && key != "glue"
                        && key != "example"
                        && key != "version"
                    {
                        match QuillValue::from_toml(value) {
                            Ok(quill_value) => {
                                metadata.insert(key.clone(), quill_value);
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to convert field '{}': {}", key, e);
                            }
                        }
                    }
                }
            }
        }

        // Extract fields from [typst] section
        if let Some(typst_section) = quill_toml.get("typst") {
            if let toml::Value::Table(table) = typst_section {
                for (key, value) in table {
                    match QuillValue::from_toml(value) {
                        Ok(quill_value) => {
                            metadata.insert(format!("typst_{}", key), quill_value);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to convert typst field '{}': {}", key, e);
                        }
                    }
                }
            }
        }

        // Parse field schemas from top-level [fields] section
        if let Some(fields_section) = quill_toml.get("fields") {
            if let toml::Value::Table(fields_table) = fields_section {
                for (field_name, field_schema) in fields_table {
                    match QuillValue::from_toml(field_schema) {
                        Ok(quill_value) => match FieldSchema::from_quill_value(&quill_value) {
                            Ok(schema) => {
                                field_schemas.insert(field_name.clone(), schema);
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to parse field schema '{}': {}",
                                    field_name, e
                                );
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to convert field schema '{}': {}",
                                field_name, e
                            );
                        }
                    }
                }
            }
        }

        // Read the template content from glue file (if specified)
        let template_content = if let Some(ref glue_file_name) = glue_file {
            let glue_bytes = root
                .get_file(glue_file_name)
                .ok_or_else(|| format!("Glue file '{}' not found in file tree", glue_file_name))?;

            String::from_utf8(glue_bytes.to_vec())
                .map_err(|e| format!("Glue file '{}' is not valid UTF-8: {}", glue_file_name, e))?
        } else {
            // If no glue file specified, use empty string (JSON glue will be used)
            String::new()
        };

        // Read the markdown template content if specified
        let template_content_opt = if let Some(ref template_file_name) = template_file {
            root.get_file(template_file_name).and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .map_err(|e| {
                        eprintln!(
                            "Warning: Template file '{}' is not valid UTF-8: {}",
                            template_file_name, e
                        );
                        e
                    })
                    .ok()
            })
        } else {
            None
        };

        let quill = Quill {
            glue_template: template_content,
            metadata,
            name: quill_name,
            backend,
            glue_file,
            example: template_content_opt,
            field_schemas,
            files: root,
        };

        // Automatically validate the quill upon creation
        quill.validate()?;

        Ok(quill)
    }

    /// Create a Quill from a JSON representation
    ///
    /// Parses a JSON string into an in-memory file tree and validates it. The
    /// precise JSON contract is documented in `designs/QUILL_DESIGN.md`.
    /// The JSON format MUST have a root object with a `files` key. The optional
    /// `metadata` key provides additional metadata that overrides defaults.
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        use serde_json::Value as JsonValue;

        let json: JsonValue =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let obj = json.as_object().ok_or_else(|| "Root must be an object")?;

        // Extract metadata (optional)
        let default_name = obj
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Extract files (required)
        let files_obj = obj
            .get("files")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "Missing or invalid 'files' key")?;

        // Parse file tree
        let mut root_files = HashMap::new();
        for (key, value) in files_obj {
            root_files.insert(key.clone(), FileTreeNode::from_json_value(value)?);
        }

        let root = FileTreeNode::Directory { files: root_files };

        // Create Quill from tree
        Self::from_tree(root, default_name)
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

    /// Get the list of typst packages to download, if specified in Quill.toml
    pub fn typst_packages(&self) -> Vec<String> {
        self.metadata
            .get("typst_packages")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Validate the quill structure
    pub fn validate(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        // Check that glue file exists in memory (if specified)
        if let Some(ref glue_file) = self.glue_file {
            if !self.files.file_exists(glue_file) {
                return Err(format!("Glue file '{}' does not exist", glue_file).into());
            }
        }
        Ok(())
    }

    /// Get file contents by path (relative to quill root)
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&[u8]> {
        self.files.get_file(path)
    }

    /// Check if a file exists in memory
    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.file_exists(path)
    }

    /// Check if a directory exists in memory
    pub fn dir_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.dir_exists(path)
    }

    /// List files in a directory (non-recursive, returns file names only)
    pub fn list_files<P: AsRef<Path>>(&self, path: P) -> Vec<String> {
        self.files.list_files(path)
    }

    /// List subdirectories in a directory (non-recursive, returns directory names only)
    pub fn list_subdirectories<P: AsRef<Path>>(&self, path: P) -> Vec<String> {
        self.files.list_subdirectories(path)
    }

    /// List all files in a directory (returns paths relative to quill root)
    pub fn list_directory<P: AsRef<Path>>(&self, dir_path: P) -> Vec<PathBuf> {
        let dir_path = dir_path.as_ref();
        let filenames = self.files.list_files(dir_path);

        // Convert filenames to full paths
        filenames
            .iter()
            .map(|name| {
                if dir_path == Path::new("") {
                    PathBuf::from(name)
                } else {
                    dir_path.join(name)
                }
            })
            .collect()
    }

    /// List all directories in a directory (returns paths relative to quill root)
    pub fn list_directories<P: AsRef<Path>>(&self, dir_path: P) -> Vec<PathBuf> {
        let dir_path = dir_path.as_ref();
        let subdirs = self.files.list_subdirectories(dir_path);

        // Convert subdirectory names to full paths
        subdirs
            .iter()
            .map(|name| {
                if dir_path == Path::new("") {
                    PathBuf::from(name)
                } else {
                    dir_path.join(name)
                }
            })
            .collect()
    }

    /// Get all files matching a pattern (supports glob-style wildcards)
    pub fn find_files<P: AsRef<Path>>(&self, pattern: P) -> Vec<PathBuf> {
        let pattern_str = pattern.as_ref().to_string_lossy();
        let mut matches = Vec::new();

        // Compile the glob pattern
        let glob_pattern = match glob::Pattern::new(&pattern_str) {
            Ok(pat) => pat,
            Err(_) => return matches, // Invalid pattern returns empty results
        };

        // Recursively search the tree for matching files
        self.find_files_recursive(&self.files, Path::new(""), &glob_pattern, &mut matches);

        matches.sort();
        matches
    }

    /// Helper method to recursively search for files matching a pattern
    fn find_files_recursive(
        &self,
        node: &FileTreeNode,
        current_path: &Path,
        pattern: &glob::Pattern,
        matches: &mut Vec<PathBuf>,
    ) {
        match node {
            FileTreeNode::File { .. } => {
                let path_str = current_path.to_string_lossy();
                if pattern.matches(&path_str) {
                    matches.push(current_path.to_path_buf());
                }
            }
            FileTreeNode::Directory { files } => {
                for (name, child_node) in files {
                    let child_path = if current_path == Path::new("") {
                        PathBuf::from(name)
                    } else {
                        current_path.join(name)
                    };
                    self.find_files_recursive(child_node, &child_path, pattern, matches);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_quillignore_parsing() {
        let ignore_content = r#"
# This is a comment
*.tmp
target/
node_modules/
.git/
"#;
        let ignore = QuillIgnore::from_content(ignore_content);
        assert_eq!(ignore.patterns.len(), 4);
        assert!(ignore.patterns.contains(&"*.tmp".to_string()));
        assert!(ignore.patterns.contains(&"target/".to_string()));
    }

    #[test]
    fn test_quillignore_matching() {
        let ignore = QuillIgnore::new(vec![
            "*.tmp".to_string(),
            "target/".to_string(),
            "node_modules/".to_string(),
            ".git/".to_string(),
        ]);

        // Test file patterns
        assert!(ignore.is_ignored("test.tmp"));
        assert!(ignore.is_ignored("path/to/file.tmp"));
        assert!(!ignore.is_ignored("test.txt"));

        // Test directory patterns
        assert!(ignore.is_ignored("target"));
        assert!(ignore.is_ignored("target/debug"));
        assert!(ignore.is_ignored("target/debug/deps"));
        assert!(!ignore.is_ignored("src/target.rs"));

        assert!(ignore.is_ignored("node_modules"));
        assert!(ignore.is_ignored("node_modules/package"));
        assert!(!ignore.is_ignored("my_node_modules"));
    }

    #[test]
    fn test_in_memory_file_system() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test files
        fs::write(
            quill_dir.join("Quill.toml"),
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"",
        )
        .unwrap();
        fs::write(quill_dir.join("glue.typ"), "test template").unwrap();

        let assets_dir = quill_dir.join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(assets_dir.join("test.txt"), "asset content").unwrap();

        let packages_dir = quill_dir.join("packages");
        fs::create_dir_all(&packages_dir).unwrap();
        fs::write(packages_dir.join("package.typ"), "package content").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test file access
        assert!(quill.file_exists("glue.typ"));
        assert!(quill.file_exists("assets/test.txt"));
        assert!(quill.file_exists("packages/package.typ"));
        assert!(!quill.file_exists("nonexistent.txt"));

        // Test file content
        let asset_content = quill.get_file("assets/test.txt").unwrap();
        assert_eq!(asset_content, b"asset content");

        // Test directory listing
        let asset_files = quill.list_directory("assets");
        assert_eq!(asset_files.len(), 1);
        assert!(asset_files.contains(&PathBuf::from("assets/test.txt")));
    }

    #[test]
    fn test_quillignore_integration() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create .quillignore
        fs::write(quill_dir.join(".quillignore"), "*.tmp\ntarget/\n").unwrap();

        // Create test files
        fs::write(
            quill_dir.join("Quill.toml"),
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"",
        )
        .unwrap();
        fs::write(quill_dir.join("glue.typ"), "test template").unwrap();
        fs::write(quill_dir.join("should_ignore.tmp"), "ignored").unwrap();

        let target_dir = quill_dir.join("target");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("debug.txt"), "also ignored").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that ignored files are not loaded
        assert!(quill.file_exists("glue.typ"));
        assert!(!quill.file_exists("should_ignore.tmp"));
        assert!(!quill.file_exists("target/debug.txt"));
    }

    #[test]
    fn test_find_files_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test directory structure
        fs::write(
            quill_dir.join("Quill.toml"),
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"",
        )
        .unwrap();
        fs::write(quill_dir.join("glue.typ"), "template").unwrap();

        let assets_dir = quill_dir.join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(assets_dir.join("image.png"), "png data").unwrap();
        fs::write(assets_dir.join("data.json"), "json data").unwrap();

        let fonts_dir = assets_dir.join("fonts");
        fs::create_dir_all(&fonts_dir).unwrap();
        fs::write(fonts_dir.join("font.ttf"), "font data").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test pattern matching
        let all_assets = quill.find_files("assets/*");
        assert!(all_assets.len() >= 3); // At least image.png, data.json, fonts/font.ttf

        let typ_files = quill.find_files("*.typ");
        assert_eq!(typ_files.len(), 1);
        assert!(typ_files.contains(&PathBuf::from("glue.typ")));
    }

    #[test]
    fn test_new_standardized_toml_format() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test files using new standardized format
        let toml_content = r#"[Quill]
name = "my-custom-quill"
backend = "typst"
glue = "custom_glue.typ"
description = "Test quill with new format"
author = "Test Author"
"#;
        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(
            quill_dir.join("custom_glue.typ"),
            "= Custom Template\n\nThis is a custom template.",
        )
        .unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that name comes from TOML, not directory
        assert_eq!(quill.name, "my-custom-quill");

        // Test that glue file is set correctly
        assert_eq!(quill.glue_file, Some("custom_glue.typ".to_string()));

        // Test that backend is in metadata
        assert!(quill.metadata.contains_key("backend"));
        if let Some(backend_val) = quill.metadata.get("backend") {
            if let Some(backend_str) = backend_val.as_str() {
                assert_eq!(backend_str, "typst");
            } else {
                panic!("Backend value is not a string");
            }
        }

        // Test that other fields are in metadata (but not version)
        assert!(quill.metadata.contains_key("description"));
        assert!(quill.metadata.contains_key("author"));
        assert!(!quill.metadata.contains_key("version")); // version should be excluded

        // Test that glue template content is loaded correctly
        assert!(quill.glue_template.contains("Custom Template"));
        assert!(quill.glue_template.contains("custom template"));
    }

    #[test]
    fn test_typst_packages_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        let toml_content = r#"
[Quill]
name = "test-quill"
backend = "typst"
glue = "glue.typ"
description = "Test quill for packages"

[typst]
packages = ["@preview/bubble:0.2.2", "@preview/example:1.0.0"]
"#;

        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(quill_dir.join("glue.typ"), "test").unwrap();

        let quill = Quill::from_path(quill_dir).unwrap();
        let packages = quill.typst_packages();

        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0], "@preview/bubble:0.2.2");
        assert_eq!(packages[1], "@preview/example:1.0.0");
    }

    #[test]
    fn test_template_loading() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test files with example specified
        let toml_content = r#"[Quill]
name = "test-with-template"
backend = "typst"
glue = "glue.typ"
example = "example.md"
description = "Test quill with template"
"#;
        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(quill_dir.join("glue.typ"), "glue content").unwrap();
        fs::write(
            quill_dir.join("example.md"),
            "---\ntitle: Test\n---\n\nThis is a test template.",
        )
        .unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that example content is loaded and includes some the text
        assert!(quill.example.is_some());
        let example = quill.example.unwrap();
        assert!(example.contains("title: Test"));
        assert!(example.contains("This is a test template"));

        // Test that glue template is still loaded
        assert_eq!(quill.glue_template, "glue content");
    }

    #[test]
    fn test_template_optional() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test files without example specified
        let toml_content = r#"[Quill]
name = "test-without-template"
backend = "typst"
glue = "glue.typ"
description = "Test quill without template"
"#;
        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(quill_dir.join("glue.typ"), "glue content").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that example fields are None
        assert_eq!(quill.example, None);

        // Test that glue template is still loaded
        assert_eq!(quill.glue_template, "glue content");
    }

    #[test]
    fn test_from_tree() {
        // Create a simple in-memory file tree
        let mut root_files = HashMap::new();

        // Add Quill.toml
        let quill_toml = r#"[Quill]
name = "test-from-tree"
backend = "typst"
glue = "glue.typ"
description = "A test quill from tree"
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        // Add glue file
        let glue_content = "= Test Template\n\nThis is a test.";
        root_files.insert(
            "glue.typ".to_string(),
            FileTreeNode::File {
                contents: glue_content.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };

        // Create Quill from tree
        let quill = Quill::from_tree(root, Some("test-from-tree".to_string())).unwrap();

        // Validate the quill
        assert_eq!(quill.name, "test-from-tree");
        assert_eq!(quill.glue_file, Some("glue.typ".to_string()));
        assert_eq!(quill.glue_template, glue_content);
        assert!(quill.metadata.contains_key("backend"));
        assert!(quill.metadata.contains_key("description"));
    }

    #[test]
    fn test_from_tree_with_template() {
        let mut root_files = HashMap::new();

        // Add Quill.toml with example specified
        let quill_toml = r#"[Quill]
name = "test-tree-template"
backend = "typst"
glue = "glue.typ"
example = "template.md"
description = "Test tree with template"
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        // Add glue file
        root_files.insert(
            "glue.typ".to_string(),
            FileTreeNode::File {
                contents: b"glue content".to_vec(),
            },
        );

        // Add template file
        let template_content = "# {{ title }}\n\n{{ body }}";
        root_files.insert(
            "template.md".to_string(),
            FileTreeNode::File {
                contents: template_content.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };

        // Create Quill from tree
        let quill = Quill::from_tree(root, None).unwrap();

        // Validate template is loaded
        assert_eq!(quill.example, Some(template_content.to_string()));
    }

    #[test]
    fn test_from_json() {
        // Create JSON representation of a Quill using new format
        let json_str = r#"{
            "metadata": {
                "name": "test-from-json"
            },
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"test-from-json\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill from JSON\"\n"
                },
                "glue.typ": {
                    "contents": "= Test Glue\n\nThis is test content."
                }
            }
        }"#;

        // Create Quill from JSON
        let quill = Quill::from_json(json_str).unwrap();

        // Validate the quill
        assert_eq!(quill.name, "test-from-json");
        assert_eq!(quill.glue_file, Some("glue.typ".to_string()));
        assert!(quill.glue_template.contains("Test Glue"));
        assert!(quill.metadata.contains_key("backend"));
    }

    #[test]
    fn test_from_json_with_byte_array() {
        // Create JSON with byte array representation using new format
        let json_str = r#"{
            "files": {
                "Quill.toml": {
                    "contents": [91, 81, 117, 105, 108, 108, 93, 10, 110, 97, 109, 101, 32, 61, 32, 34, 116, 101, 115, 116, 34, 10, 98, 97, 99, 107, 101, 110, 100, 32, 61, 32, 34, 116, 121, 112, 115, 116, 34, 10, 103, 108, 117, 101, 32, 61, 32, 34, 103, 108, 117, 101, 46, 116, 121, 112, 34, 10, 100, 101, 115, 99, 114, 105, 112, 116, 105, 111, 110, 32, 61, 32, 34, 84, 101, 115, 116, 32, 113, 117, 105, 108, 108, 34, 10]
                },
                "glue.typ": {
                    "contents": "test glue"
                }
            }
        }"#;

        // Create Quill from JSON
        let quill = Quill::from_json(json_str).unwrap();

        // Validate the quill was created
        assert_eq!(quill.name, "test");
        assert_eq!(quill.glue_file, Some("glue.typ".to_string()));
    }

    #[test]
    fn test_from_json_missing_files() {
        // JSON without files field should fail
        let json_str = r#"{
            "metadata": {
                "name": "test"
            }
        }"#;

        let result = Quill::from_json(json_str);
        assert!(result.is_err());
        // Should fail because there's no 'files' key
        assert!(result.unwrap_err().to_string().contains("files"));
    }

    #[test]
    fn test_from_json_tree_structure() {
        // Test the new tree structure format
        let json_str = r#"{
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"test-tree-json\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test tree JSON\"\n"
                },
                "glue.typ": {
                    "contents": "= Test Glue\n\nTree structure content."
                }
            }
        }"#;

        let quill = Quill::from_json(json_str).unwrap();

        assert_eq!(quill.name, "test-tree-json");
        assert!(quill.glue_template.contains("Tree structure content"));
        assert!(quill.metadata.contains_key("backend"));
    }

    #[test]
    fn test_from_json_nested_tree_structure() {
        // Test nested directories in tree structure
        let json_str = r#"{
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"nested-test\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Nested test\"\n"
                },
                "glue.typ": {
                    "contents": "glue"
                },
                "src": {
                    "main.rs": {
                        "contents": "fn main() {}"
                    },
                    "lib.rs": {
                        "contents": "// lib"
                    }
                }
            }
        }"#;

        let quill = Quill::from_json(json_str).unwrap();

        assert_eq!(quill.name, "nested-test");
        // Verify nested files are accessible
        assert!(quill.file_exists("src/main.rs"));
        assert!(quill.file_exists("src/lib.rs"));

        let main_rs = quill.get_file("src/main.rs").unwrap();
        assert_eq!(main_rs, b"fn main() {}");
    }

    #[test]
    fn test_from_tree_structure_direct() {
        // Test using from_tree_structure directly
        let mut root_files = HashMap::new();

        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents:
                    b"[Quill]\nname = \"direct-tree\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Direct tree test\"\n"
                        .to_vec(),
            },
        );

        root_files.insert(
            "glue.typ".to_string(),
            FileTreeNode::File {
                contents: b"glue content".to_vec(),
            },
        );

        // Add a nested directory
        let mut src_files = HashMap::new();
        src_files.insert(
            "main.rs".to_string(),
            FileTreeNode::File {
                contents: b"fn main() {}".to_vec(),
            },
        );

        root_files.insert(
            "src".to_string(),
            FileTreeNode::Directory { files: src_files },
        );

        let root = FileTreeNode::Directory { files: root_files };

        let quill = Quill::from_tree(root, None).unwrap();

        assert_eq!(quill.name, "direct-tree");
        assert!(quill.file_exists("src/main.rs"));
        assert!(quill.file_exists("glue.typ"));
    }

    #[test]
    fn test_from_json_with_metadata_override() {
        // Test that metadata key overrides name from Quill.toml
        let json_str = r#"{
            "metadata": {
                "name": "override-name"
            },
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"toml-name\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"TOML name test\"\n"
                },
                "glue.typ": {
                    "contents": "= glue"
                }
            }
        }"#;

        let quill = Quill::from_json(json_str).unwrap();
        // Metadata name should be used as default, but Quill.toml takes precedence
        // when from_tree is called
        assert_eq!(quill.name, "toml-name");
    }

    #[test]
    fn test_from_json_empty_directory() {
        // Test that empty directories are supported
        let json_str = r#"{
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"empty-dir-test\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Empty directory test\"\n"
                },
                "glue.typ": {
                    "contents": "glue"
                },
                "empty_dir": {}
            }
        }"#;

        let quill = Quill::from_json(json_str).unwrap();
        assert_eq!(quill.name, "empty-dir-test");
        assert!(quill.dir_exists("empty_dir"));
        assert!(!quill.file_exists("empty_dir"));
    }

    #[test]
    fn test_dir_exists_and_list_apis() {
        let mut root_files = HashMap::new();

        // Add Quill.toml
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: b"[Quill]\nname = \"test\"\nbackend = \"typst\"\nglue = \"glue.typ\"\ndescription = \"Test quill\"\n"
                    .to_vec(),
            },
        );

        // Add glue file
        root_files.insert(
            "glue.typ".to_string(),
            FileTreeNode::File {
                contents: b"glue content".to_vec(),
            },
        );

        // Add assets directory with files
        let mut assets_files = HashMap::new();
        assets_files.insert(
            "logo.png".to_string(),
            FileTreeNode::File {
                contents: vec![137, 80, 78, 71],
            },
        );
        assets_files.insert(
            "icon.svg".to_string(),
            FileTreeNode::File {
                contents: b"<svg></svg>".to_vec(),
            },
        );

        // Add subdirectory in assets
        let mut fonts_files = HashMap::new();
        fonts_files.insert(
            "font.ttf".to_string(),
            FileTreeNode::File {
                contents: b"font data".to_vec(),
            },
        );
        assets_files.insert(
            "fonts".to_string(),
            FileTreeNode::Directory { files: fonts_files },
        );

        root_files.insert(
            "assets".to_string(),
            FileTreeNode::Directory {
                files: assets_files,
            },
        );

        // Add empty directory
        root_files.insert(
            "empty".to_string(),
            FileTreeNode::Directory {
                files: HashMap::new(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };
        let quill = Quill::from_tree(root, None).unwrap();

        // Test dir_exists
        assert!(quill.dir_exists("assets"));
        assert!(quill.dir_exists("assets/fonts"));
        assert!(quill.dir_exists("empty"));
        assert!(!quill.dir_exists("nonexistent"));
        assert!(!quill.dir_exists("glue.typ")); // file, not directory

        // Test file_exists
        assert!(quill.file_exists("glue.typ"));
        assert!(quill.file_exists("assets/logo.png"));
        assert!(quill.file_exists("assets/fonts/font.ttf"));
        assert!(!quill.file_exists("assets")); // directory, not file

        // Test list_files
        let root_files_list = quill.list_files("");
        assert_eq!(root_files_list.len(), 2); // Quill.toml and glue.typ
        assert!(root_files_list.contains(&"Quill.toml".to_string()));
        assert!(root_files_list.contains(&"glue.typ".to_string()));

        let assets_files_list = quill.list_files("assets");
        assert_eq!(assets_files_list.len(), 2); // logo.png and icon.svg
        assert!(assets_files_list.contains(&"logo.png".to_string()));
        assert!(assets_files_list.contains(&"icon.svg".to_string()));

        // Test list_subdirectories
        let root_subdirs = quill.list_subdirectories("");
        assert_eq!(root_subdirs.len(), 2); // assets and empty
        assert!(root_subdirs.contains(&"assets".to_string()));
        assert!(root_subdirs.contains(&"empty".to_string()));

        let assets_subdirs = quill.list_subdirectories("assets");
        assert_eq!(assets_subdirs.len(), 1); // fonts
        assert!(assets_subdirs.contains(&"fonts".to_string()));

        let empty_subdirs = quill.list_subdirectories("empty");
        assert_eq!(empty_subdirs.len(), 0);
    }

    #[test]
    fn test_field_schemas_parsing() {
        let mut root_files = HashMap::new();

        // Add Quill.toml with field schemas
        let quill_toml = r#"[Quill]
name = "taro"
backend = "typst"
glue = "glue.typ"
example = "taro.md"
description = "Test template for field schemas"

[fields]
author = {description = "Author of document", required = true}
ice_cream = {description = "favorite ice cream flavor"}
title = {description = "title of document", required = true}
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        // Add glue file
        let glue_content = "= Test Template\n\nThis is a test.";
        root_files.insert(
            "glue.typ".to_string(),
            FileTreeNode::File {
                contents: glue_content.as_bytes().to_vec(),
            },
        );

        // Add template file
        root_files.insert(
            "taro.md".to_string(),
            FileTreeNode::File {
                contents: b"# Template".to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };

        // Create Quill from tree
        let quill = Quill::from_tree(root, Some("taro".to_string())).unwrap();

        // Validate field schemas were parsed
        assert_eq!(quill.field_schemas.len(), 3);
        assert!(quill.field_schemas.contains_key("author"));
        assert!(quill.field_schemas.contains_key("ice_cream"));
        assert!(quill.field_schemas.contains_key("title"));

        // Verify author field schema
        let author_schema = quill.field_schemas.get("author").unwrap();
        assert_eq!(author_schema.description, "Author of document");
        assert_eq!(author_schema.required, true);

        // Verify ice_cream field schema (no required field, should default to false)
        let ice_cream_schema = quill.field_schemas.get("ice_cream").unwrap();
        assert_eq!(ice_cream_schema.description, "favorite ice cream flavor");
        assert_eq!(ice_cream_schema.required, false);

        // Verify title field schema
        let title_schema = quill.field_schemas.get("title").unwrap();
        assert_eq!(title_schema.description, "title of document");
        assert_eq!(title_schema.required, true);
    }

    #[test]
    fn test_field_schema_struct() {
        // Test creating FieldSchema with minimal fields
        let schema1 = FieldSchema::new("Test description".to_string());
        assert_eq!(schema1.description, "Test description");
        assert_eq!(schema1.required, false);
        assert_eq!(schema1.r#type, None);
        assert_eq!(schema1.example, None);
        assert_eq!(schema1.default, None);

        // Test parsing FieldSchema from YAML with all fields
        let yaml_str = r#"
description: "Full field schema"
required: true
type: "string"
example: "Example value"
default: "Default value"
"#;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
        let quill_value = QuillValue::from_yaml(yaml_value).unwrap();
        let schema2 = FieldSchema::from_quill_value(&quill_value).unwrap();
        assert_eq!(schema2.description, "Full field schema");
        assert_eq!(schema2.required, true);
        assert_eq!(schema2.r#type, Some("string".to_string()));
        assert_eq!(
            schema2.example.as_ref().and_then(|v| v.as_str()),
            Some("Example value")
        );
        assert_eq!(
            schema2.default.as_ref().and_then(|v| v.as_str()),
            Some("Default value")
        );

        // Test converting FieldSchema back to QuillValue
        let quill_value = schema2.to_quill_value();
        let obj = quill_value.as_object().unwrap();
        assert_eq!(
            obj.get("description").unwrap().as_str().unwrap(),
            "Full field schema"
        );
        assert_eq!(obj.get("required").unwrap().as_bool().unwrap(), true);
        assert_eq!(obj.get("type").unwrap().as_str().unwrap(), "string");
    }

    #[test]
    fn test_quill_without_glue_file() {
        // Test creating a Quill without specifying a glue file
        let mut root_files = HashMap::new();

        // Add Quill.toml without glue field
        let quill_toml = r#"[Quill]
name = "test-no-glue"
backend = "typst"
description = "Test quill without glue file"
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };

        // Create Quill from tree
        let quill = Quill::from_tree(root, None).unwrap();

        // Validate that glue_file is None
        assert_eq!(quill.glue_file, None);
        // Validate that glue_template is empty (will use JSON glue)
        assert_eq!(quill.glue_template, "");
        assert_eq!(quill.name, "test-no-glue");
    }
}
