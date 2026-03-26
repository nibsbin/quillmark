//! Quill template bundle types and implementations.

mod config;
mod ignore;
mod load;
mod query;
mod tree;
mod types;

pub use config::QuillConfig;
pub use ignore::QuillIgnore;
pub use tree::FileTreeNode;
pub use types::{
    field_key, ui_key, CardSchema, FieldSchema, FieldType, UiContainerSchema, UiFieldSchema,
};

use std::collections::HashMap;

use crate::value::QuillValue;

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

#[cfg(test)]
mod tests;
