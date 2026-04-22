//! Quill source bundle types and implementations.

mod config;
mod formats;
mod ignore;
mod load;
mod query;
mod schema;
mod schema_yaml;
mod tree;
mod types;
pub(crate) mod validation;

pub use config::{CoercionError, QuillConfig};
pub use ignore::QuillIgnore;
pub use schema::build_transform_schema;
pub use tree::FileTreeNode;
pub use types::{
    field_key, ui_key, CardSchema, FieldSchema, FieldType, UiContainerSchema, UiFieldSchema,
};

use std::collections::HashMap;

use crate::value::QuillValue;

/// A quill source bundle — pure data parsed from an authored quill directory.
///
/// A `QuillSource` is the file-bundle, config, and metadata; it has no rendering
/// ability. The engine composes a `QuillSource` with a resolved backend into a
/// renderable `Quill` (see `quillmark::Quill`).
#[derive(Clone)]
pub struct QuillSource {
    /// Quill-specific metadata
    pub metadata: HashMap<String, QuillValue>,
    /// Name of the quill
    pub name: String,
    /// Backend identifier (e.g., "typst")
    pub backend_id: String,
    /// Plate template content (optional)
    pub plate: Option<String>,
    /// Markdown template content (optional)
    pub example: Option<String>,
    /// Parsed configuration — the authoritative schema model.
    pub config: QuillConfig,
    /// Cached default values extracted from config (for performance)
    pub defaults: HashMap<String, QuillValue>,
    /// Cached example values extracted from config (for performance)
    pub examples: HashMap<String, Vec<QuillValue>>,
    /// In-memory file system (tree structure)
    pub files: FileTreeNode,
}

impl std::fmt::Debug for QuillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuillSource")
            .field("name", &self.name)
            .field("backend_id", &self.backend_id)
            .field(
                "plate",
                &self.plate.as_ref().map(|s| format!("<{} bytes>", s.len())),
            )
            .field("example", &self.example.is_some())
            .field("files", &"<FileTreeNode>")
            .finish()
    }
}

#[cfg(test)]
mod tests;
