//! Quill template bundle types and implementations.

use std::collections::HashMap;
use std::error::Error as StdError;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::value::QuillValue;

/// Semantic constants for field schema keys used in parsing and JSON Schema generation.
/// Using constants provides IDE support (find references, autocomplete) and ensures
/// consistency between parsing and output.
pub mod field_key {
    /// Field name identifier
    pub const NAME: &str = "name";
    /// Short label for the field
    pub const TITLE: &str = "title";
    /// Field type (string, number, boolean, array, etc.)
    pub const TYPE: &str = "type";
    /// Detailed field description
    pub const DESCRIPTION: &str = "description";
    /// Default value for the field
    pub const DEFAULT: &str = "default";
    /// Example values for the field
    pub const EXAMPLES: &str = "examples";
    /// UI-specific metadata
    pub const UI: &str = "ui";
    /// Whether the field is required
    pub const REQUIRED: &str = "required";
    /// Enum values for string fields
    pub const ENUM: &str = "enum";
    /// Date format specifier (JSON Schema)
    pub const FORMAT: &str = "format";
}

/// Semantic constants for UI schema keys
pub mod ui_key {
    /// Group name for field organization
    pub const GROUP: &str = "group";
    /// Display order within the UI
    pub const ORDER: &str = "order";
}

/// UI-specific metadata for field rendering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiSchema {
    /// Group name for organizing fields (e.g., "Personal Info", "Preferences")
    pub group: Option<String>,
    /// Order of the field in the UI (automatically generated based on field position in Quill.toml)
    pub order: Option<i32>,
}

/// Schema definition for a card type (composable content blocks)
#[derive(Debug, Clone, PartialEq)]
pub struct CardSchema {
    /// Card type name (e.g., "indorsements")
    pub name: String,
    /// Short label for the card type
    pub title: Option<String>,
    /// Detailed description of this card type
    pub description: String,
    /// Field definitions for this card type
    pub fields: HashMap<String, FieldSchema>,
}

/// Field type hint enum for type-safe field type definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// String type (alias: "string")
    String,
    /// String type (alias: "str")
    #[serde(alias = "str")]
    Str,
    /// Numeric type
    Number,
    /// Boolean type
    Boolean,
    /// Array type
    Array,
    /// Dictionary/object type
    Dict,
    /// Date type (formatted as string with date format)
    Date,
    /// DateTime type (formatted as string with date-time format)
    DateTime,
}

impl FieldType {
    /// Parse a FieldType from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "string" => Some(FieldType::String),
            "str" => Some(FieldType::Str),
            "number" => Some(FieldType::Number),
            "boolean" => Some(FieldType::Boolean),
            "array" => Some(FieldType::Array),
            "dict" => Some(FieldType::Dict),
            "date" => Some(FieldType::Date),
            "datetime" => Some(FieldType::DateTime),
            _ => None,
        }
    }

    /// Get the canonical string representation for this type
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Str => "str",
            FieldType::Number => "number",
            FieldType::Boolean => "boolean",
            FieldType::Array => "array",
            FieldType::Dict => "dict",
            FieldType::Date => "date",
            FieldType::DateTime => "datetime",
        }
    }
}

/// Schema definition for a template field
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSchema {
    pub name: String,
    /// Short label for the field (used in JSON Schema title)
    pub title: Option<String>,
    /// Field type hint (e.g., String, Number, Boolean, Dict, Array)
    pub r#type: Option<FieldType>,
    /// Detailed description of the field (used in JSON Schema description)
    pub description: String,
    /// Default value for the field
    pub default: Option<QuillValue>,
    /// Example values for the field
    pub examples: Option<QuillValue>,
    /// UI-specific metadata for rendering
    pub ui: Option<UiSchema>,
    /// Whether this field is required (fields are optional by default)
    pub required: bool,
    /// Enum values for string fields (restricts valid values)
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldSchemaDef {
    pub title: Option<String>,
    pub r#type: Option<FieldType>,
    #[serde(default)]
    pub description: String,
    pub default: Option<QuillValue>,
    pub examples: Option<QuillValue>,
    pub ui: Option<UiSchema>,
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}

impl FieldSchema {
    /// Create a new FieldSchema with default values
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            title: None,
            r#type: None,
            description,
            default: None,
            examples: None,
            ui: None,
            required: false,
            enum_values: None,
        }
    }

    /// Parse a FieldSchema from a QuillValue
    pub fn from_quill_value(key: String, value: &QuillValue) -> Result<Self, String> {
        let def: FieldSchemaDef = serde_json::from_value(value.clone().into_json())
            .map_err(|e| format!("Failed to parse field schema: {}", e))?;

        Ok(Self {
            name: key,
            title: def.title,
            r#type: def.r#type,
            description: def.description,
            default: def.default,
            examples: def.examples,
            ui: def.ui,
            required: def.required,
            enum_values: def.enum_values,
        })
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

    pub fn print_tree(&self) -> String {
        self.__print_tree("", "", true)
    }

    pub fn __print_tree(&self, name: &str, prefix: &str, is_last: bool) -> String {
        let mut result = String::new();

        // Choose the appropriate tree characters
        let connector = if is_last { "└── " } else { "├── " };
        let extension = if is_last { "    " } else { "│   " };

        match self {
            FileTreeNode::File { .. } => {
                result.push_str(&format!("{}{}{}\n", prefix, connector, name));
            }
            FileTreeNode::Directory { files } => {
                // Add trailing slash for directories like `tree` does
                result.push_str(&format!("{}{}{}/\n", prefix, connector, name));

                let child_prefix = format!("{}{}", prefix, extension);
                let count = files.len();

                for (i, (child_name, node)) in files.iter().enumerate() {
                    let is_last_child = i == count - 1;
                    result.push_str(&node.__print_tree(child_name, &child_prefix, is_last_child));
                }
            }
        }

        result
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
        if let Some(pattern_prefix) = pattern.strip_suffix('/') {
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
    /// Quill-specific metadata
    pub metadata: HashMap<String, QuillValue>,
    /// Name of the quill
    pub name: String,
    /// Backend identifier (e.g., "typst")
    pub backend: String,
    /// Plate template content (optional)
    pub plate: Option<String>,
    /// Markdown template content (optional)
    pub example: Option<String>,
    /// Field JSON schema (single source of truth for schema and defaults)
    pub schema: QuillValue,
    /// Cached default values extracted from schema (for performance)
    pub defaults: HashMap<String, QuillValue>,
    /// Cached example values extracted from schema (for performance)
    pub examples: HashMap<String, Vec<QuillValue>>,
    /// In-memory file system (tree structure)
    pub files: FileTreeNode,
}

/// Quill configuration extracted from Quill.toml
#[derive(Debug, Clone)]
pub struct QuillConfig {
    /// Human-readable name
    pub name: String,
    /// Description of the quill
    pub description: String,
    /// Backend identifier (e.g., "typst")
    pub backend: String,
    /// Semantic version of the quill
    pub version: Option<String>,
    /// Author of the quill
    pub author: Option<String>,
    /// Example markdown file
    pub example_file: Option<String>,
    /// Plate file
    pub plate_file: Option<String>,
    /// Field schemas
    pub fields: HashMap<String, FieldSchema>,
    /// Card schemas (composable content blocks)
    pub cards: HashMap<String, CardSchema>,
    /// Additional metadata from [Quill] section (excluding standard fields)
    pub metadata: HashMap<String, QuillValue>,
    /// Typst-specific configuration from `[typst]` section
    pub typst_config: HashMap<String, QuillValue>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CardSchemaDef {
    pub title: Option<String>,
    #[serde(default)]
    pub description: String,
    pub fields: Option<toml::value::Table>,
}

impl QuillConfig {
    /// Parse fields from a TOML table, assigning ui.order based on key_order.
    ///
    /// This helper ensures consistent field ordering logic for both top-level
    /// fields and card fields.
    ///
    /// # Arguments
    /// * `fields_table` - The TOML table containing field definitions
    /// * `key_order` - Vector of field names in their definition order (from toml_edit)
    /// * `context` - Context string for error messages (e.g., "field" or "card 'indorsement' field")
    fn parse_fields_with_order(
        fields_table: &toml::value::Table,
        key_order: &[String],
        context: &str,
    ) -> HashMap<String, FieldSchema> {
        let mut fields = HashMap::new();
        let mut fallback_counter = 0;

        for (field_name, field_value) in fields_table {
            // Determine order from key_order, or use fallback counter
            let order = if let Some(idx) = key_order.iter().position(|k| k == field_name) {
                idx as i32
            } else {
                let o = key_order.len() as i32 + fallback_counter;
                fallback_counter += 1;
                o
            };

            match QuillValue::from_toml(field_value) {
                Ok(quill_value) => {
                    match FieldSchema::from_quill_value(field_name.clone(), &quill_value) {
                        Ok(mut schema) => {
                            // Always set ui.order based on position
                            if schema.ui.is_none() {
                                schema.ui = Some(UiSchema {
                                    group: None,
                                    order: Some(order),
                                });
                            } else if let Some(ui) = &mut schema.ui {
                                ui.order = Some(order);
                            }

                            fields.insert(field_name.clone(), schema);
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to parse {} '{}': {}",
                                context, field_name, e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to convert {} '{}': {}",
                        context, field_name, e
                    );
                }
            }
        }

        fields
    }

    /// Extract key order from a toml_edit table at the given path.
    ///
    /// Returns a vector of key names in their definition order.
    fn get_key_order(doc: Option<&toml_edit::DocumentMut>, path: &[&str]) -> Vec<String> {
        doc.and_then(|d| {
            let mut current: &dyn toml_edit::TableLike = d.as_table();
            for segment in path {
                current = current.get(*segment)?.as_table()?;
            }
            Some(current.iter().map(|(k, _)| k.to_string()).collect())
        })
        .unwrap_or_default()
    }

    /// Parse QuillConfig from TOML content
    pub fn from_toml(toml_content: &str) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        let quill_toml: toml::Value = toml::from_str(toml_content)
            .map_err(|e| format!("Failed to parse Quill.toml: {}", e))?;

        // Parse with toml_edit to preserve field order
        let toml_edit_doc = toml_content.parse::<toml_edit::DocumentMut>().ok();

        // Extract [Quill] section (required)
        let quill_section = quill_toml
            .get("Quill")
            .ok_or("Missing required [Quill] section in Quill.toml")?;

        // Extract required fields
        let name = quill_section
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required 'name' field in [Quill] section")?
            .to_string();

        let backend = quill_section
            .get("backend")
            .and_then(|v| v.as_str())
            .ok_or("Missing required 'backend' field in [Quill] section")?
            .to_string();

        let description = quill_section
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or("Missing required 'description' field in [Quill] section")?;

        if description.trim().is_empty() {
            return Err("'description' field in [Quill] section cannot be empty".into());
        }
        let description = description.to_string();

        // Extract optional fields
        let version = quill_section
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let author = quill_section
            .get("author")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let example_file = quill_section
            .get("example_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let plate_file = quill_section
            .get("plate_file")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract additional metadata from [Quill] section (excluding standard fields)
        let mut metadata = HashMap::new();
        if let toml::Value::Table(table) = quill_section {
            for (key, value) in table {
                // Skip standard fields that are stored in dedicated struct fields
                if key != "name"
                    && key != "backend"
                    && key != "description"
                    && key != "version"
                    && key != "author"
                    && key != "example_file"
                    && key != "plate_file"
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

        // Extract [typst] section (optional)
        let mut typst_config = HashMap::new();
        if let Some(toml::Value::Table(table)) = quill_toml.get("typst") {
            for (key, value) in table {
                match QuillValue::from_toml(value) {
                    Ok(quill_value) => {
                        typst_config.insert(key.clone(), quill_value);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to convert typst field '{}': {}", key, e);
                    }
                }
            }
        }

        // Extract [fields] section (optional) using shared helper
        let fields = if let Some(toml::Value::Table(fields_table)) = quill_toml.get("fields") {
            let field_order = Self::get_key_order(toml_edit_doc.as_ref(), &["fields"]);
            Self::parse_fields_with_order(fields_table, &field_order, "field schema")
        } else {
            HashMap::new()
        };

        // Extract [cards] section (optional)
        // Extract [cards] section (optional)
        let mut cards: HashMap<String, CardSchema> = HashMap::new();
        if let Some(cards_value) = quill_toml.get("cards") {
            // Need to parse cards manually to some extent because we need to process 'fields'
            // and check for collisions

            let cards_table = cards_value
                .as_table()
                .ok_or("'cards' section must be a table")?;

            for (card_name, card_value) in cards_table {
                // Parse card basic info using serde
                let card_def: CardSchemaDef = card_value
                    .clone()
                    .try_into()
                    .map_err(|e| format!("Failed to parse card '{}': {}", card_name, e))?;

                // Parse card fields using shared helper
                let card_fields = if let Some(card_fields_table) = &card_def.fields {
                    let card_field_order = Self::get_key_order(
                        toml_edit_doc.as_ref(),
                        &["cards", card_name, "fields"],
                    );

                    Self::parse_fields_with_order(
                        card_fields_table,
                        &card_field_order,
                        &format!("card '{}' field", card_name),
                    )
                } else {
                    HashMap::new()
                };

                let card_schema = CardSchema {
                    name: card_name.clone(),
                    title: card_def.title,
                    description: card_def.description,
                    fields: card_fields,
                };

                cards.insert(card_name.clone(), card_schema);
            }
        }

        Ok(QuillConfig {
            name,
            description,
            backend,
            version,
            author,
            example_file,
            plate_file,
            fields,
            cards,
            metadata,
            typst_config,
        })
    }
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
    /// * `_default_name` - Unused parameter kept for API compatibility (name always from Quill.toml)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Quill.toml is not found in the file tree
    /// - Quill.toml is not valid UTF-8 or TOML
    /// - The plate file specified in Quill.toml is not found or not valid UTF-8
    /// - Validation fails
    pub fn from_tree(
        root: FileTreeNode,
        _default_name: Option<String>,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        // Read Quill.toml
        let quill_toml_bytes = root
            .get_file("Quill.toml")
            .ok_or("Quill.toml not found in file tree")?;

        let quill_toml_content = String::from_utf8(quill_toml_bytes.to_vec())
            .map_err(|e| format!("Quill.toml is not valid UTF-8: {}", e))?;

        // Parse TOML into QuillConfig
        let config = QuillConfig::from_toml(&quill_toml_content)?;

        // Construct Quill from QuillConfig
        Self::from_config(config, root)
    }

    /// Create a Quill from a QuillConfig and file tree
    ///
    /// This method constructs a Quill from a parsed QuillConfig and validates
    /// all file references.
    ///
    /// # Arguments
    ///
    /// * `config` - The parsed QuillConfig
    /// * `root` - The root node of the file tree
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The plate file specified in config is not found or not valid UTF-8
    /// - The example file specified in config is not found or not valid UTF-8
    /// - Schema generation fails
    fn from_config(
        config: QuillConfig,
        root: FileTreeNode,
    ) -> Result<Self, Box<dyn StdError + Send + Sync>> {
        // Build metadata from config
        let mut metadata = config.metadata.clone();

        // Add backend to metadata
        metadata.insert(
            "backend".to_string(),
            QuillValue::from_json(serde_json::Value::String(config.backend.clone())),
        );

        // Add description to metadata
        metadata.insert(
            "description".to_string(),
            QuillValue::from_json(serde_json::Value::String(config.description.clone())),
        );

        // Add author if present
        if let Some(ref author) = config.author {
            metadata.insert(
                "author".to_string(),
                QuillValue::from_json(serde_json::Value::String(author.clone())),
            );
        }

        // Add typst config to metadata with typst_ prefix
        for (key, value) in &config.typst_config {
            metadata.insert(format!("typst_{}", key), value.clone());
        }

        // Build JSON schema from field and card schemas
        let schema = crate::schema::build_schema(&config.fields, &config.cards)
            .map_err(|e| format!("Failed to build JSON schema from field schemas: {}", e))?;

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

        // Read the markdown example content if specified
        let example_content = if let Some(ref example_file_name) = config.example_file {
            root.get_file(example_file_name).and_then(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .map_err(|e| {
                        eprintln!(
                            "Warning: Example file '{}' is not valid UTF-8: {}",
                            example_file_name, e
                        );
                        e
                    })
                    .ok()
            })
        } else {
            None
        };

        // Extract and cache defaults and examples from schema for performance
        let defaults = crate::schema::extract_defaults_from_schema(&schema);
        let examples = crate::schema::extract_examples_from_schema(&schema);

        let quill = Quill {
            metadata,
            name: config.name,
            backend: config.backend,
            plate: plate_content,
            example: example_content,
            schema,
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
        use serde_json::Value as JsonValue;

        let json: JsonValue =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let obj = json.as_object().ok_or("Root must be an object")?;

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
            .ok_or("Missing or invalid 'files' key")?;

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

    /// Get default values from the cached schema defaults
    ///
    /// Returns a reference to the pre-computed defaults HashMap that was extracted
    /// during Quill construction. This is more efficient than re-parsing the schema.
    ///
    /// This is used by `ParsedDocument::with_defaults()` to apply default values
    /// to missing fields.
    pub fn extract_defaults(&self) -> &HashMap<String, QuillValue> {
        &self.defaults
    }

    /// Get example values from the cached schema examples
    ///
    /// Returns a reference to the pre-computed examples HashMap that was extracted
    /// during Quill construction. This is more efficient than re-parsing the schema.
    pub fn extract_examples(&self) -> &HashMap<String, Vec<QuillValue>> {
        &self.examples
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
        Self::find_files_recursive(&self.files, Path::new(""), &glob_pattern, &mut matches);

        matches.sort();
        matches
    }

    /// Helper method to recursively search for files matching a pattern
    fn find_files_recursive(
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
                    Self::find_files_recursive(child_node, &child_path, pattern, matches);
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
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"",
        )
        .unwrap();
        fs::write(quill_dir.join("plate.typ"), "test plate").unwrap();

        let assets_dir = quill_dir.join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(assets_dir.join("test.txt"), "asset content").unwrap();

        let packages_dir = quill_dir.join("packages");
        fs::create_dir_all(&packages_dir).unwrap();
        fs::write(packages_dir.join("package.typ"), "package content").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test file access
        assert!(quill.file_exists("plate.typ"));
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
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"",
        )
        .unwrap();
        fs::write(quill_dir.join("plate.typ"), "test template").unwrap();
        fs::write(quill_dir.join("should_ignore.tmp"), "ignored").unwrap();

        let target_dir = quill_dir.join("target");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("debug.txt"), "also ignored").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that ignored files are not loaded
        assert!(quill.file_exists("plate.typ"));
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
            "[Quill]\nname = \"test\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"",
        )
        .unwrap();
        fs::write(quill_dir.join("plate.typ"), "template").unwrap();

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
        assert!(typ_files.contains(&PathBuf::from("plate.typ")));
    }

    #[test]
    fn test_new_standardized_toml_format() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test files using new standardized format
        let toml_content = r#"[Quill]
name = "my-custom-quill"
backend = "typst"
plate_file = "custom_plate.typ"
description = "Test quill with new format"
author = "Test Author"
"#;
        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(
            quill_dir.join("custom_plate.typ"),
            "= Custom Template\n\nThis is a custom template.",
        )
        .unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that name comes from TOML, not directory
        assert_eq!(quill.name, "my-custom-quill");

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

        // Test that plate template content is loaded correctly
        assert!(quill.plate.unwrap().contains("Custom Template"));
    }

    #[test]
    fn test_typst_packages_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        let toml_content = r#"
[Quill]
name = "test-quill"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill for packages"

[typst]
packages = ["@preview/bubble:0.2.2", "@preview/example:1.0.0"]
"#;

        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(quill_dir.join("plate.typ"), "test").unwrap();

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
plate_file = "plate.typ"
example_file = "example.md"
description = "Test quill with template"
"#;
        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(quill_dir.join("plate.typ"), "plate content").unwrap();
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

        // Test that plate template is still loaded
        assert_eq!(quill.plate.unwrap(), "plate content");
    }

    #[test]
    fn test_template_optional() {
        let temp_dir = TempDir::new().unwrap();
        let quill_dir = temp_dir.path();

        // Create test files without example specified
        let toml_content = r#"[Quill]
name = "test-without-template"
backend = "typst"
plate_file = "plate.typ"
description = "Test quill without template"
"#;
        fs::write(quill_dir.join("Quill.toml"), toml_content).unwrap();
        fs::write(quill_dir.join("plate.typ"), "plate content").unwrap();

        // Load quill
        let quill = Quill::from_path(quill_dir).unwrap();

        // Test that example fields are None
        assert_eq!(quill.example, None);

        // Test that plate template is still loaded
        assert_eq!(quill.plate.unwrap(), "plate content");
    }

    #[test]
    fn test_from_tree() {
        // Create a simple in-memory file tree
        let mut root_files = HashMap::new();

        // Add Quill.toml
        let quill_toml = r#"[Quill]
name = "test-from-tree"
backend = "typst"
plate_file = "plate.typ"
description = "A test quill from tree"
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        // Add plate file
        let plate_content = "= Test Template\n\nThis is a test.";
        root_files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: plate_content.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };

        // Create Quill from tree
        let quill = Quill::from_tree(root, Some("test-from-tree".to_string())).unwrap();

        // Validate the quill
        assert_eq!(quill.name, "test-from-tree");
        assert_eq!(quill.plate.unwrap(), plate_content);
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
plate_file = "plate.typ"
example_file = "template.md"
description = "Test tree with template"
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        // Add plate file
        root_files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: b"plate content".to_vec(),
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
                    "contents": "[Quill]\nname = \"test-from-json\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill from JSON\"\n"
                },
                "plate.typ": {
                    "contents": "= Test Plate\n\nThis is test content."
                }
            }
        }"#;

        // Create Quill from JSON
        let quill = Quill::from_json(json_str).unwrap();

        // Validate the quill
        assert_eq!(quill.name, "test-from-json");
        assert!(quill.plate.unwrap().contains("Test Plate"));
        assert!(quill.metadata.contains_key("backend"));
    }

    #[test]
    fn test_from_json_with_byte_array() {
        // Create JSON with byte array representation using new format
        let json_str = r#"{
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"test\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n"
                },
                "plate.typ": {
                    "contents": "test plate"
                }
            }
        }"#;

        // Create Quill from JSON
        let quill = Quill::from_json(json_str).unwrap();

        // Validate the quill was created
        assert_eq!(quill.name, "test");
        assert_eq!(quill.plate.unwrap(), "test plate");
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
                    "contents": "[Quill]\nname = \"test-tree-json\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test tree JSON\"\n"
                },
                "plate.typ": {
                    "contents": "= Test Plate\n\nTree structure content."
                }
            }
        }"#;

        let quill = Quill::from_json(json_str).unwrap();

        assert_eq!(quill.name, "test-tree-json");
        assert!(quill.plate.unwrap().contains("Tree structure content"));
        assert!(quill.metadata.contains_key("backend"));
    }

    #[test]
    fn test_from_json_nested_tree_structure() {
        // Test nested directories in tree structure
        let json_str = r#"{
            "files": {
                "Quill.toml": {
                    "contents": "[Quill]\nname = \"nested-test\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Nested test\"\n"
                },
                "plate.typ": {
                    "contents": "plate"
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
                    b"[Quill]\nname = \"direct-tree\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Direct tree test\"\n"
                        .to_vec(),
            },
        );

        root_files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: b"plate content".to_vec(),
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
        assert!(quill.file_exists("plate.typ"));
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
                    "contents": "[Quill]\nname = \"toml-name\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"TOML name test\"\n"
                },
                "plate.typ": {
                    "contents": "= plate"
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
                    "contents": "[Quill]\nname = \"empty-dir-test\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Empty directory test\"\n"
                },
                "plate.typ": {
                    "contents": "plate"
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
                contents: b"[Quill]\nname = \"test\"\nbackend = \"typst\"\nplate_file = \"plate.typ\"\ndescription = \"Test quill\"\n"
                    .to_vec(),
            },
        );

        // Add plate file
        root_files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: b"plate content".to_vec(),
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
        assert!(!quill.dir_exists("plate.typ")); // file, not directory

        // Test file_exists
        assert!(quill.file_exists("plate.typ"));
        assert!(quill.file_exists("assets/logo.png"));
        assert!(quill.file_exists("assets/fonts/font.ttf"));
        assert!(!quill.file_exists("assets")); // directory, not file

        // Test list_files
        let root_files_list = quill.list_files("");
        assert_eq!(root_files_list.len(), 2); // Quill.toml and plate.typ
        assert!(root_files_list.contains(&"Quill.toml".to_string()));
        assert!(root_files_list.contains(&"plate.typ".to_string()));

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
plate_file = "plate.typ"
example_file = "taro.md"
description = "Test template for field schemas"

[fields]
author = {description = "Author of document" }
ice_cream = {description = "favorite ice cream flavor"}
title = {description = "title of document" }
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        // Add plate file
        let plate_content = "= Test Template\n\nThis is a test.";
        root_files.insert(
            "plate.typ".to_string(),
            FileTreeNode::File {
                contents: plate_content.as_bytes().to_vec(),
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
        assert_eq!(quill.schema["properties"].as_object().unwrap().len(), 3);
        assert!(quill.schema["properties"]
            .as_object()
            .unwrap()
            .contains_key("author"));
        assert!(quill.schema["properties"]
            .as_object()
            .unwrap()
            .contains_key("ice_cream"));
        assert!(quill.schema["properties"]
            .as_object()
            .unwrap()
            .contains_key("title"));

        // Verify author field schema
        let author_schema = quill.schema["properties"]["author"].as_object().unwrap();
        assert_eq!(author_schema["description"], "Author of document");

        // Verify ice_cream field schema (no required field, should default to false)
        let ice_cream_schema = quill.schema["properties"]["ice_cream"].as_object().unwrap();
        assert_eq!(ice_cream_schema["description"], "favorite ice cream flavor");

        // Verify title field schema
        let title_schema = quill.schema["properties"]["title"].as_object().unwrap();
        assert_eq!(title_schema["description"], "title of document");
    }

    #[test]
    fn test_field_schema_struct() {
        // Test creating FieldSchema with minimal fields
        let schema1 = FieldSchema::new("test_name".to_string(), "Test description".to_string());
        assert_eq!(schema1.description, "Test description");
        assert_eq!(schema1.r#type, None);
        assert_eq!(schema1.examples, None);
        assert_eq!(schema1.default, None);

        // Test parsing FieldSchema from YAML with all fields
        let yaml_str = r#"
description: "Full field schema"
type: "string"
examples:
  - "Example value"
default: "Default value"
"#;
        let quill_value = QuillValue::from_yaml_str(yaml_str).unwrap();
        let schema2 = FieldSchema::from_quill_value("test_name".to_string(), &quill_value).unwrap();
        assert_eq!(schema2.name, "test_name");
        assert_eq!(schema2.description, "Full field schema");
        assert_eq!(schema2.r#type, Some(FieldType::String));
        assert_eq!(
            schema2
                .examples
                .as_ref()
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str()),
            Some("Example value")
        );
        assert_eq!(
            schema2.default.as_ref().and_then(|v| v.as_str()),
            Some("Default value")
        );
    }

    #[test]
    fn test_quill_without_plate_file() {
        // Test creating a Quill without specifying a plate file
        let mut root_files = HashMap::new();

        // Add Quill.toml without plate field
        let quill_toml = r#"[Quill]
name = "test-no-plate"
backend = "typst"
description = "Test quill without plate file"
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

        // Validate that plate is null (will use auto plate)
        assert!(quill.plate.clone().is_none());
        assert_eq!(quill.name, "test-no-plate");
    }

    #[test]
    fn test_quill_config_from_toml() {
        // Test parsing QuillConfig from TOML content
        let toml_content = r#"[Quill]
name = "test-config"
backend = "typst"
description = "Test configuration parsing"
version = "1.0.0"
author = "Test Author"
plate_file = "plate.typ"
example_file = "example.md"

[typst]
packages = ["@preview/bubble:0.2.2"]

[fields]
title = {description = "Document title", type = "string"}
author = {description = "Document author"}
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();

        // Verify required fields
        assert_eq!(config.name, "test-config");
        assert_eq!(config.backend, "typst");
        assert_eq!(config.description, "Test configuration parsing");

        // Verify optional fields
        assert_eq!(config.version, Some("1.0.0".to_string()));
        assert_eq!(config.author, Some("Test Author".to_string()));
        assert_eq!(config.plate_file, Some("plate.typ".to_string()));
        assert_eq!(config.example_file, Some("example.md".to_string()));

        // Verify typst config
        assert!(config.typst_config.contains_key("packages"));

        // Verify field schemas
        assert_eq!(config.fields.len(), 2);
        assert!(config.fields.contains_key("title"));
        assert!(config.fields.contains_key("author"));

        let title_field = &config.fields["title"];
        assert_eq!(title_field.description, "Document title");
        assert_eq!(title_field.r#type, Some(FieldType::String));
    }

    #[test]
    fn test_quill_config_missing_required_fields() {
        // Test that missing required fields result in error
        let toml_missing_name = r#"[Quill]
backend = "typst"
description = "Missing name"
"#;
        let result = QuillConfig::from_toml(toml_missing_name);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required 'name'"));

        let toml_missing_backend = r#"[Quill]
name = "test"
description = "Missing backend"
"#;
        let result = QuillConfig::from_toml(toml_missing_backend);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required 'backend'"));

        let toml_missing_description = r#"[Quill]
name = "test"
backend = "typst"
"#;
        let result = QuillConfig::from_toml(toml_missing_description);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required 'description'"));
    }

    #[test]
    fn test_quill_config_empty_description() {
        // Test that empty description results in error
        let toml_empty_description = r#"[Quill]
name = "test"
backend = "typst"
description = "   "
"#;
        let result = QuillConfig::from_toml(toml_empty_description);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("description' field in [Quill] section cannot be empty"));
    }

    #[test]
    fn test_quill_config_missing_quill_section() {
        // Test that missing [Quill] section results in error
        let toml_no_section = r#"[fields]
title = {description = "Title"}
"#;
        let result = QuillConfig::from_toml(toml_no_section);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required [Quill] section"));
    }

    #[test]
    fn test_quill_from_config_metadata() {
        // Test that QuillConfig metadata flows through to Quill
        let mut root_files = HashMap::new();

        let quill_toml = r#"[Quill]
name = "metadata-test"
backend = "typst"
description = "Test metadata flow"
author = "Test Author"
custom_field = "custom_value"

[typst]
packages = ["@preview/bubble:0.2.2"]
"#;
        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };
        let quill = Quill::from_tree(root, None).unwrap();

        // Verify metadata includes backend and description
        assert!(quill.metadata.contains_key("backend"));
        assert!(quill.metadata.contains_key("description"));
        assert!(quill.metadata.contains_key("author"));

        // Verify custom field is in metadata
        assert!(quill.metadata.contains_key("custom_field"));
        assert_eq!(
            quill.metadata.get("custom_field").unwrap().as_str(),
            Some("custom_value")
        );

        // Verify typst config with typst_ prefix
        assert!(quill.metadata.contains_key("typst_packages"));
    }

    #[test]
    fn test_extract_defaults_method() {
        // Test the extract_defaults method on Quill
        let mut root_files = HashMap::new();

        let quill_toml = r#"[Quill]
name = "defaults-test"
backend = "typst"
description = "Test defaults extraction"

[fields]
title = {description = "Title"}
author = {description = "Author", default = "Anonymous"}
status = {description = "Status", default = "draft"}
"#;

        root_files.insert(
            "Quill.toml".to_string(),
            FileTreeNode::File {
                contents: quill_toml.as_bytes().to_vec(),
            },
        );

        let root = FileTreeNode::Directory { files: root_files };
        let quill = Quill::from_tree(root, None).unwrap();

        // Extract defaults
        let defaults = quill.extract_defaults();

        // Verify only fields with defaults are returned
        assert_eq!(defaults.len(), 2);
        assert!(!defaults.contains_key("title")); // no default
        assert!(defaults.contains_key("author"));
        assert!(defaults.contains_key("status"));

        // Verify default values
        assert_eq!(defaults.get("author").unwrap().as_str(), Some("Anonymous"));
        assert_eq!(defaults.get("status").unwrap().as_str(), Some("draft"));
    }

    #[test]
    fn test_field_order_preservation() {
        let toml_content = r#"[Quill]
name = "order-test"
backend = "typst"
description = "Test field order"

[fields]
first = {description = "First field"}
second = {description = "Second field"}
third = {description = "Third field", ui = {group = "Test Group"}}
fourth = {description = "Fourth field"}
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();

        // Check that fields have correct order based on TOML position
        // Order is automatically generated based on field position

        let first = config.fields.get("first").unwrap();
        assert_eq!(first.ui.as_ref().unwrap().order, Some(0));

        let second = config.fields.get("second").unwrap();
        assert_eq!(second.ui.as_ref().unwrap().order, Some(1));

        let third = config.fields.get("third").unwrap();
        assert_eq!(third.ui.as_ref().unwrap().order, Some(2));
        assert_eq!(
            third.ui.as_ref().unwrap().group,
            Some("Test Group".to_string())
        );

        let fourth = config.fields.get("fourth").unwrap();
        assert_eq!(fourth.ui.as_ref().unwrap().order, Some(3));
    }

    #[test]
    fn test_quill_with_all_ui_properties() {
        let toml_content = r#"[Quill]
name = "full-ui-test"
backend = "typst"
description = "Test all UI properties"

[fields.author]
description = "The full name of the document author"
type = "str"

[fields.author.ui]
group = "Author Info"
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();

        let author_field = &config.fields["author"];
        let ui = author_field.ui.as_ref().unwrap();
        assert_eq!(ui.group, Some("Author Info".to_string()));
        assert_eq!(ui.order, Some(0)); // First field should have order 0
    }
    #[test]
    fn test_field_schema_with_title_and_description() {
        // Test parsing field with new schema format (title + description, no tooltip)
        let yaml = r#"
title: "Field Title"
description: "Detailed field description"
type: "string"
examples:
  - "Example value"
ui:
  group: "Test Group"
"#;
        let quill_value = QuillValue::from_yaml_str(yaml).unwrap();
        let schema = FieldSchema::from_quill_value("test_field".to_string(), &quill_value).unwrap();

        assert_eq!(schema.title, Some("Field Title".to_string()));
        assert_eq!(schema.description, "Detailed field description");

        assert_eq!(
            schema
                .examples
                .as_ref()
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str()),
            Some("Example value")
        );

        let ui = schema.ui.as_ref().unwrap();
        assert_eq!(ui.group, Some("Test Group".to_string()));
    }

    #[test]
    fn test_parse_card_field_type() {
        // Test that FieldSchema no longer supports type = "card" (cards are in CardSchema now)
        let yaml = r#"
type: "string"
title: "Simple Field"
description: "A simple string field"
"#;
        let quill_value = QuillValue::from_yaml_str(yaml).unwrap();
        let schema =
            FieldSchema::from_quill_value("simple_field".to_string(), &quill_value).unwrap();

        assert_eq!(schema.name, "simple_field");
        assert_eq!(schema.r#type, Some(FieldType::String));
        assert_eq!(schema.title, Some("Simple Field".to_string()));
        assert_eq!(schema.description, "A simple string field");
    }

    #[test]
    fn test_parse_card_with_fields_in_toml() {
        // Test parsing [cards] section with [cards.X.fields.Y] syntax
        let toml_content = r#"[Quill]
name = "cards-fields-test"
backend = "typst"
description = "Test [cards.X.fields.Y] syntax"

[cards.endorsements]
title = "Endorsements"
description = "Chain of endorsements"

[cards.endorsements.fields.name]
type = "string"
title = "Endorser Name"
description = "Name of the endorsing official"
required = true

[cards.endorsements.fields.org]
type = "string"
title = "Organization"
description = "Endorser's organization"
default = "Unknown"
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();

        // Verify the card was parsed into config.cards
        assert!(config.cards.contains_key("endorsements"));
        let card = config.cards.get("endorsements").unwrap();

        assert_eq!(card.name, "endorsements");
        assert_eq!(card.title, Some("Endorsements".to_string()));
        assert_eq!(card.description, "Chain of endorsements");

        // Verify card fields
        assert_eq!(card.fields.len(), 2);

        let name_field = card.fields.get("name").unwrap();
        assert_eq!(name_field.r#type, Some(FieldType::String));
        assert_eq!(name_field.title, Some("Endorser Name".to_string()));
        assert!(name_field.required);

        let org_field = card.fields.get("org").unwrap();
        assert_eq!(org_field.r#type, Some(FieldType::String));
        assert!(org_field.default.is_some());
        assert_eq!(
            org_field.default.as_ref().unwrap().as_str(),
            Some("Unknown")
        );
    }

    #[test]
    fn test_field_schema_rejects_unknown_keys() {
        // Test that unknown keys like "items" are rejected (strict mode)
        let yaml = r#"
type: "string"
description: "A string field"
items:
  sub_field:
    type: "string"
    description: "Nested field"
"#;
        let quill_value = QuillValue::from_yaml_str(yaml).unwrap();

        let result = FieldSchema::from_quill_value("author".to_string(), &quill_value);

        // The parsing should fail due to deny_unknown_fields
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("unknown field `items`"), "Error was: {}", err);
    }

    #[test]
    fn test_quill_config_with_cards_section() {
        let toml_content = r#"[Quill]
name = "cards-test"
backend = "typst"
description = "Test [cards] section"

[fields.regular]
description = "Regular field"
type = "string"

[cards.indorsements]
title = "Routing Indorsements"
description = "Chain of endorsements"

[cards.indorsements.fields.name]
title = "Name"
type = "string"
description = "Name field"
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();

        // Check regular field
        assert!(config.fields.contains_key("regular"));
        let regular = config.fields.get("regular").unwrap();
        assert_eq!(regular.r#type, Some(FieldType::String));

        // Check card is in config.cards (not config.fields)
        assert!(config.cards.contains_key("indorsements"));
        let card = config.cards.get("indorsements").unwrap();
        assert_eq!(card.title, Some("Routing Indorsements".to_string()));
        assert_eq!(card.description, "Chain of endorsements");
        assert!(card.fields.contains_key("name"));
    }

    #[test]
    fn test_quill_config_cards_empty_fields() {
        // Test that cards with no fields section are valid
        let toml_content = r#"[Quill]
name = "cards-empty-fields-test"
backend = "typst"
description = "Test cards without fields"

[cards.myscope]
description = "My scope"
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();
        let card = config.cards.get("myscope").unwrap();
        assert_eq!(card.name, "myscope");
        assert_eq!(card.description, "My scope");
        assert!(card.fields.is_empty());
    }

    #[test]
    fn test_quill_config_allows_card_collision() {
        // Test that scope name colliding with field name is ALLOWED
        let toml_content = r#"[Quill]
name = "collision-test"
backend = "typst"
description = "Test collision"

[fields.conflict]
description = "Field"
type = "string"

[cards.conflict]
description = "Card"
"#;

        let result = QuillConfig::from_toml(toml_content);
        if let Err(e) = &result {
            panic!(
                "Card name collision should be allowed, but got error: {}",
                e
            );
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.fields.contains_key("conflict"));
        assert!(config.cards.contains_key("conflict"));
    }

    #[test]
    fn test_quill_config_ordering_with_cards() {
        // Test that fields have proper UI ordering (cards no longer have card-level ordering)
        let toml_content = r#"[Quill]
name = "ordering-test"
backend = "typst"
description = "Test ordering"

[fields.first]
description = "First"

[cards.second]
description = "Second"

[cards.second.fields.card_field]
description = "A card field"

[fields.zero]
description = "Zero"
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();

        let first = config.fields.get("first").unwrap();
        let zero = config.fields.get("zero").unwrap();
        let second = config.cards.get("second").unwrap();

        // Check field ordering
        let ord_first = first.ui.as_ref().unwrap().order.unwrap();
        let ord_zero = zero.ui.as_ref().unwrap().order.unwrap();

        // Within fields, "first" is before "zero"
        assert!(ord_first < ord_zero);
        assert_eq!(ord_first, 0);
        assert_eq!(ord_zero, 1);

        // Card fields should also have ordering
        let card_field = second.fields.get("card_field").unwrap();
        let ord_card_field = card_field.ui.as_ref().unwrap().order.unwrap();
        assert_eq!(ord_card_field, 0); // First (and only) field in this card
    }
    #[test]
    fn test_card_field_order_preservation() {
        // Test that card fields preserve definition order (not alphabetical)
        // defined: z_first, then a_second
        // alphabetical: a_second, then z_first
        let toml_content = r#"[Quill]
name = "card-order-test"
backend = "typst"
description = "Test card field order"

[cards.mycard]
description = "Test card"

[cards.mycard.fields.z_first]
type = "string"
description = "Defined first"

[cards.mycard.fields.a_second]
type = "string"
description = "Defined second"
"#;

        let config = QuillConfig::from_toml(toml_content).unwrap();
        let card = config.cards.get("mycard").unwrap();

        let z_first = card.fields.get("z_first").unwrap();
        let a_second = card.fields.get("a_second").unwrap();

        // Check orders
        let z_order = z_first.ui.as_ref().unwrap().order.unwrap();
        let a_order = a_second.ui.as_ref().unwrap().order.unwrap();

        // If strict file order is preserved:
        // z_first should be 0, a_second should be 1
        assert_eq!(z_order, 0, "z_first should be 0 (defined first)");
        assert_eq!(a_order, 1, "a_second should be 1 (defined second)");
    }
}
