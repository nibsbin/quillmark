//! # Quillmark Core Overview
//!
//! Core types and functionality for the Quillmark template-first Markdown rendering system.
//!
//! ## Features
//!
//! This crate provides the foundational types and traits for Quillmark:
//!
//! - **Parsing**: YAML frontmatter extraction with Extended YAML Metadata Standard support
//! - **Templating**: MiniJinja-based template composition with stable filter API
//! - **Template model**: [`Quill`] type for managing template bundles with in-memory file system
//! - **Backend trait**: Extensible interface for implementing output format backends
//! - **Error handling**: Structured diagnostics with source location tracking
//! - **Utilities**: TOML⇄YAML conversion helpers
//!
//! ## Quick Start
//!
//! ```no_run
//! use quillmark_core::{decompose, Quill};
//!
//! // Parse markdown with frontmatter
//! let markdown = "---\ntitle: Example\n---\n\n# Content";
//! let doc = decompose(markdown).unwrap();
//!
//! // Load a quill template
//! let quill = Quill::from_path("path/to/quill").unwrap();
//! ```
//!
//! ## Architecture
//!
//! The crate is organized into four main modules:
//!
//! - [`parse`]: Markdown parsing with YAML frontmatter support
//! - [`templating`]: Template composition using MiniJinja
//! - [`backend`]: Backend trait for output format implementations
//! - [`error`]: Structured error handling and diagnostics
//!
//! ## Further Reading
//!
//! - [PARSE.md](https://github.com/nibsbin/quillmark/blob/main/quillmark-core/docs/designs/PARSE.md) - Detailed parsing documentation
//! - [Examples](https://github.com/nibsbin/quillmark/tree/main/examples) - Working examples

use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::{Path, PathBuf};

pub mod parse;
pub use parse::{decompose, ParsedDocument, BODY_FIELD};

pub mod templating;
pub use templating::{Glue, TemplateError};

pub mod backend;
pub use backend::Backend;

pub mod error;
pub use error::{Diagnostic, Location, RenderError, RenderResult, Severity};

/// Output formats supported by backends. See [module docs](self) for examples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum OutputFormat {
    /// Plain text output
    Txt,
    /// Scalable Vector Graphics output
    Svg,
    /// Portable Document Format output
    Pdf,
}

/// An artifact produced by rendering. See [module docs](self) for examples.
#[derive(Debug)]
pub struct Artifact {
    /// The binary content of the artifact
    pub bytes: Vec<u8>,
    /// The format of the output
    pub output_format: OutputFormat,
}

/// Internal rendering options. See [module docs](self) for examples.
#[derive(Debug)]
pub struct RenderOptions {
    /// Optional output format specification
    pub output_format: Option<OutputFormat>,
}

/// A file entry in the in-memory file system
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The file contents as bytes
    pub contents: Vec<u8>,
    /// The file path relative to the quill root
    pub path: PathBuf,
    /// Whether this is a directory entry
    pub is_dir: bool,
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

/// A quill template bundle. See [module docs](self) for examples.
#[derive(Debug, Clone)]
pub struct Quill {
    /// The template content
    pub glue_template: String,
    /// Quill-specific metadata
    pub metadata: HashMap<String, serde_yaml::Value>,
    /// Base path for resolving relative paths
    pub base_path: PathBuf,
    /// Name of the quill
    pub name: String,
    /// Glue template file name
    pub glue_file: String,
    /// Markdown template file name (optional)
    pub template_file: Option<String>,
    /// Markdown template content (optional)
    pub template: Option<String>,
    /// In-memory file system
    pub files: HashMap<PathBuf, FileEntry>,
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

        // Load all files into memory
        let mut files = HashMap::new();
        Self::load_directory_recursive(path, path, &mut files, &ignore)?;

        // Create Quill from the file tree
        Self::from_tree(files, Some(path.to_path_buf()), Some(name))
    }

    /// Create a Quill from a tree of files (authoritative method)
    ///
    /// This is the authoritative method for creating a Quill from an in-memory file tree.
    /// Both `from_path` and `from_json` use this method internally.
    ///
    /// # Arguments
    ///
    /// * `files` - A map of file paths to `FileEntry` objects representing the file tree
    /// * `base_path` - Optional base path for the Quill (defaults to "/")
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
        files: HashMap<PathBuf, FileEntry>,
        base_path: Option<PathBuf>,
        default_name: Option<String>,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        // Read Quill.toml
        let quill_toml_path = PathBuf::from("Quill.toml");
        let quill_toml_entry = files
            .get(&quill_toml_path)
            .ok_or("Quill.toml not found in file tree")?;

        let quill_toml_content = String::from_utf8(quill_toml_entry.contents.clone())
            .map_err(|e| format!("Quill.toml is not valid UTF-8: {}", e))?;

        let quill_toml: toml::Value = toml::from_str(&quill_toml_content)
            .map_err(|e| format!("Failed to parse Quill.toml: {}", e))?;

        let mut metadata = HashMap::new();
        let mut glue_file = "glue.typ".to_string(); // default
        let mut template_file: Option<String> = None;
        let mut quill_name = default_name.unwrap_or_else(|| "unnamed".to_string());

        // Extract fields from [Quill] section
        if let Some(quill_section) = quill_toml.get("Quill") {
            // Extract required fields: name, backend, glue, template
            if let Some(name_val) = quill_section.get("name").and_then(|v| v.as_str()) {
                quill_name = name_val.to_string();
            }

            if let Some(backend_val) = quill_section.get("backend").and_then(|v| v.as_str()) {
                match Self::toml_to_yaml_value(&toml::Value::String(backend_val.to_string())) {
                    Ok(yaml_value) => {
                        metadata.insert("backend".to_string(), yaml_value);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to convert backend field: {}", e);
                    }
                }
            }

            if let Some(glue_val) = quill_section.get("glue").and_then(|v| v.as_str()) {
                glue_file = glue_val.to_string();
            }

            if let Some(template_val) = quill_section.get("template").and_then(|v| v.as_str()) {
                template_file = Some(template_val.to_string());
            }

            // Add other fields to metadata (excluding special fields and version)
            if let toml::Value::Table(table) = quill_section {
                for (key, value) in table {
                    if key != "name"
                        && key != "backend"
                        && key != "glue"
                        && key != "template"
                        && key != "version"
                    {
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

        // Extract fields from [typst] section
        if let Some(typst_section) = quill_toml.get("typst") {
            if let toml::Value::Table(table) = typst_section {
                for (key, value) in table {
                    match Self::toml_to_yaml_value(value) {
                        Ok(yaml_value) => {
                            metadata.insert(format!("typst_{}", key), yaml_value);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to convert typst field '{}': {}", key, e);
                        }
                    }
                }
            }
        }

        // Read the template content from glue file
        let glue_path = PathBuf::from(&glue_file);
        let glue_entry = files
            .get(&glue_path)
            .ok_or_else(|| format!("Glue file '{}' not found in file tree", glue_file))?;

        let template_content = String::from_utf8(glue_entry.contents.clone())
            .map_err(|e| format!("Glue file '{}' is not valid UTF-8: {}", glue_file, e))?;

        // Read the markdown template content if specified
        let template_content_opt = if let Some(ref template_file_name) = template_file {
            let template_path = PathBuf::from(template_file_name);
            files.get(&template_path).and_then(|entry| {
                String::from_utf8(entry.contents.clone())
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
            base_path: base_path.unwrap_or_else(|| PathBuf::from("/")),
            name: quill_name,
            glue_file,
            template_file,
            template: template_content_opt,
            files,
        };

        // Automatically validate the quill upon creation
        quill.validate()?;

        Ok(quill)
    }

    /// Create a Quill from a JSON representation
    ///
    /// Parses a JSON string representing a Quill and creates a Quill instance.
    /// The JSON should have the following structure:
    ///
    /// ```json
    /// {
    ///   "name": "optional-default-name",
    ///   "base_path": "/optional/base/path",
    ///   "files": {
    ///     "Quill.toml": {
    ///       "contents": "...",  // UTF-8 string or byte array
    ///       "is_dir": false
    ///     },
    ///     "glue.typ": {
    ///       "contents": "...",
    ///       "is_dir": false
    ///     }
    ///   }
    /// }
    /// ```
    ///
    /// File contents can be either:
    /// - A UTF-8 string (recommended for text files)
    /// - An array of byte values (for binary files)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The JSON is malformed
    /// - The "files" field is missing or not an object
    /// - Any file contents are invalid
    /// - Validation fails (via `from_tree`)
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        use serde_json::Value as JsonValue;

        let json: JsonValue =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Parse files from JSON
        let files_json = json.get("files").ok_or("Missing 'files' field in JSON")?;

        let mut files = HashMap::new();
        if let JsonValue::Object(files_obj) = files_json {
            for (path_str, file_data) in files_obj {
                let path = PathBuf::from(path_str);

                let contents =
                    if let Some(content_str) = file_data.get("contents").and_then(|v| v.as_str()) {
                        // Direct UTF-8 string
                        content_str.as_bytes().to_vec()
                    } else if let Some(bytes_array) =
                        file_data.get("contents").and_then(|v| v.as_array())
                    {
                        // Array of byte values
                        bytes_array
                            .iter()
                            .filter_map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok()))
                            .collect()
                    } else {
                        return Err(format!("Invalid contents for file '{}'", path_str).into());
                    };

                let is_dir = file_data
                    .get("is_dir")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                files.insert(
                    path.clone(),
                    FileEntry {
                        contents,
                        path,
                        is_dir,
                    },
                );
            }
        } else {
            return Err("'files' field must be an object".into());
        }

        // Extract optional base_path and name from JSON
        let base_path = json
            .get("base_path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let default_name = json.get("name").and_then(|v| v.as_str()).map(String::from);

        // Create Quill from the file tree
        Self::from_tree(files, base_path, default_name)
    }

    /// Recursively load all files from a directory into memory
    fn load_directory_recursive(
        current_dir: &Path,
        base_dir: &Path,
        files: &mut HashMap<PathBuf, FileEntry>,
        ignore: &QuillIgnore,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        use std::fs;

        if !current_dir.exists() {
            return Ok(());
        }

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

            if path.is_file() {
                let contents = fs::read(&path)
                    .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

                files.insert(
                    relative_path.clone(),
                    FileEntry {
                        contents,
                        path: relative_path,
                        is_dir: false,
                    },
                );
            } else if path.is_dir() {
                // Add directory entry
                files.insert(
                    relative_path.clone(),
                    FileEntry {
                        contents: Vec::new(),
                        path: relative_path,
                        is_dir: true,
                    },
                );

                // Recursively process subdirectory
                Self::load_directory_recursive(&path, base_dir, files, ignore)?;
            }
        }

        Ok(())
    }

    /// Convert TOML value to YAML value
    pub fn toml_to_yaml_value(
        toml_val: &toml::Value,
    ) -> Result<serde_yaml::Value, Box<dyn StdError + Send + Sync>> {
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

    /// Get the list of typst packages to download, if specified in Quill.toml
    pub fn typst_packages(&self) -> Vec<String> {
        self.metadata
            .get("typst_packages")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Validate the quill structure
    pub fn validate(&self) -> Result<(), Box<dyn StdError + Send + Sync>> {
        // Check that glue file exists in memory
        let glue_path = PathBuf::from(&self.glue_file);
        if !self.files.contains_key(&glue_path) {
            return Err(format!("Glue file '{}' does not exist", self.glue_file).into());
        }
        Ok(())
    }

    /// Get file contents by path (relative to quill root)
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<&[u8]> {
        let path = path.as_ref();
        self.files.get(path).map(|entry| entry.contents.as_slice())
    }

    /// Get file entry by path (includes metadata)
    pub fn get_file_entry<P: AsRef<Path>>(&self, path: P) -> Option<&FileEntry> {
        let path = path.as_ref();
        self.files.get(path)
    }

    /// Check if a file exists in memory
    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        self.files.contains_key(path)
    }

    /// List all files in a directory (returns paths relative to quill root)
    pub fn list_directory<P: AsRef<Path>>(&self, dir_path: P) -> Vec<PathBuf> {
        let dir_path = dir_path.as_ref();
        let mut entries = Vec::new();

        for (path, entry) in &self.files {
            if let Some(parent) = path.parent() {
                if parent == dir_path && !entry.is_dir {
                    entries.push(path.clone());
                }
            } else if dir_path == Path::new("") && !entry.is_dir {
                // Files in root directory
                entries.push(path.clone());
            }
        }

        entries.sort();
        entries
    }

    /// List all directories in a directory (returns paths relative to quill root)
    pub fn list_subdirectories<P: AsRef<Path>>(&self, dir_path: P) -> Vec<PathBuf> {
        let dir_path = dir_path.as_ref();
        let mut entries = Vec::new();

        for (path, entry) in &self.files {
            if entry.is_dir {
                if let Some(parent) = path.parent() {
                    if parent == dir_path {
                        entries.push(path.clone());
                    }
                } else if dir_path == Path::new("") {
                    // Directories in root
                    entries.push(path.clone());
                }
            }
        }

        entries.sort();
        entries
    }

    /// Get all files matching a pattern (supports simple wildcards)
    pub fn find_files<P: AsRef<Path>>(&self, pattern: P) -> Vec<PathBuf> {
        let pattern_str = pattern.as_ref().to_string_lossy();
        let mut matches = Vec::new();

        for (path, entry) in &self.files {
            if !entry.is_dir {
                let path_str = path.to_string_lossy();
                if self.matches_simple_pattern(&pattern_str, &path_str) {
                    matches.push(path.clone());
                }
            }
        }

        matches.sort();
        matches
    }

    /// Simple pattern matching helper
    fn matches_simple_pattern(&self, pattern: &str, path: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') {
            return path == pattern;
        }

        // Handle directory/* patterns
        if pattern.ends_with("/*") {
            let dir_pattern = &pattern[..pattern.len() - 2];
            return path.starts_with(&format!("{}/", dir_pattern));
        }

        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
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
#[cfg(test)]
mod quill_tests {
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
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nglue = \"glue.typ\"",
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
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nglue = \"glue.typ\"",
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
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nglue = \"glue.typ\"",
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
        assert_eq!(quill.glue_file, "custom_glue.typ");

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

        // Create test files with template specified
        let toml_content = r#"[Quill]
name = "test-with-template"
backend = "typst"
glue = "glue.typ"
template = "example.md"
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

        // Test that template file name is set
        assert_eq!(quill.template_file, Some("example.md".to_string()));

        // Test that template content is loaded
        assert!(quill.template.is_some());
        let template = quill.template.unwrap();
        assert!(template.contains("title: Test"));
        assert!(template.contains("This is a test template"));

        // Test that glue template is still loaded
        assert_eq!(quill.glue_template, "glue content");
    }

    #[test]
    fn test_template_optional() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test files without template specified
        let toml_content = r#"[Quill]
name = "test-without-template"
backend = "typst"
glue = "glue.typ"
"#;
        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(quill_dir.join("glue.typ"), "glue content").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that template fields are None
        assert_eq!(quill.template_file, None);
        assert_eq!(quill.template, None);

        // Test that glue template is still loaded
        assert_eq!(quill.glue_template, "glue content");
    }

    #[test]
    fn test_from_tree() {
        // Create a simple in-memory file tree
        let mut files = HashMap::new();

        // Add Quill.toml
        let quill_toml = r#"[Quill]
name = "test-from-tree"
backend = "typst"
glue = "glue.typ"
description = "A test quill from tree"
"#;
        files.insert(
            PathBuf::from("Quill.toml"),
            FileEntry {
                contents: quill_toml.as_bytes().to_vec(),
                path: PathBuf::from("Quill.toml"),
                is_dir: false,
            },
        );

        // Add glue file
        let glue_content = "= Test Template\n\nThis is a test.";
        files.insert(
            PathBuf::from("glue.typ"),
            FileEntry {
                contents: glue_content.as_bytes().to_vec(),
                path: PathBuf::from("glue.typ"),
                is_dir: false,
            },
        );

        // Create Quill from tree
        let quill = Quill::from_tree(files, Some(PathBuf::from("/test")), None).unwrap();

        // Validate the quill
        assert_eq!(quill.name, "test-from-tree");
        assert_eq!(quill.glue_file, "glue.typ");
        assert_eq!(quill.glue_template, glue_content);
        assert_eq!(quill.base_path, PathBuf::from("/test"));
        assert!(quill.metadata.contains_key("backend"));
        assert!(quill.metadata.contains_key("description"));
    }

    #[test]
    fn test_from_tree_with_template() {
        let mut files = HashMap::new();

        // Add Quill.toml with template specified
        let quill_toml = r#"[Quill]
name = "test-tree-template"
backend = "typst"
glue = "glue.typ"
template = "template.md"
"#;
        files.insert(
            PathBuf::from("Quill.toml"),
            FileEntry {
                contents: quill_toml.as_bytes().to_vec(),
                path: PathBuf::from("Quill.toml"),
                is_dir: false,
            },
        );

        // Add glue file
        files.insert(
            PathBuf::from("glue.typ"),
            FileEntry {
                contents: b"glue content".to_vec(),
                path: PathBuf::from("glue.typ"),
                is_dir: false,
            },
        );

        // Add template file
        let template_content = "# {{ title }}\n\n{{ body }}";
        files.insert(
            PathBuf::from("template.md"),
            FileEntry {
                contents: template_content.as_bytes().to_vec(),
                path: PathBuf::from("template.md"),
                is_dir: false,
            },
        );

        // Create Quill from tree
        let quill = Quill::from_tree(files, None, None).unwrap();

        // Validate template is loaded
        assert_eq!(quill.template_file, Some("template.md".to_string()));
        assert_eq!(quill.template, Some(template_content.to_string()));
    }

    #[test]
    fn test_from_json() {
        // Create JSON representation of a Quill
        let json_str = r#"{
            "name": "test-from-json",
            "base_path": "/test/path",
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"test-from-json\"\nbackend = \"typst\"\nglue = \"glue.typ\"\n",
                    "is_dir": false
                },
                "glue.typ": {
                    "contents": "= Test Glue\n\nThis is test content.",
                    "is_dir": false
                }
            }
        }"#;

        // Create Quill from JSON
        let quill = Quill::from_json(json_str).unwrap();

        // Validate the quill
        assert_eq!(quill.name, "test-from-json");
        assert_eq!(quill.base_path, PathBuf::from("/test/path"));
        assert_eq!(quill.glue_file, "glue.typ");
        assert!(quill.glue_template.contains("Test Glue"));
        assert!(quill.metadata.contains_key("backend"));
    }

    #[test]
    fn test_from_json_with_byte_array() {
        // Create JSON with byte array representation
        let json_str = r#"{
            "files": {
                "Quill.toml": {
                    "contents": [91, 81, 117, 105, 108, 108, 93, 10, 110, 97, 109, 101, 32, 61, 32, 34, 116, 101, 115, 116, 34, 10, 98, 97, 99, 107, 101, 110, 100, 32, 61, 32, 34, 116, 121, 112, 115, 116, 34, 10, 103, 108, 117, 101, 32, 61, 32, 34, 103, 108, 117, 101, 46, 116, 121, 112, 34, 10],
                    "is_dir": false
                },
                "glue.typ": {
                    "contents": "test glue",
                    "is_dir": false
                }
            }
        }"#;

        // Create Quill from JSON
        let quill = Quill::from_json(json_str).unwrap();

        // Validate the quill was created
        assert_eq!(quill.name, "test");
        assert_eq!(quill.glue_file, "glue.typ");
    }

    #[test]
    fn test_from_json_missing_files() {
        // JSON without files field should fail
        let json_str = r#"{
            "name": "test"
        }"#;

        let result = Quill::from_json(json_str);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'files' field"));
    }
}
