//! Quill format bundle types and implementations.

mod config;
mod formats;
mod ignore;
mod load;
mod query;
mod render;
mod schema_yaml;
mod tree;
mod types;
pub(crate) mod validation;

pub use config::{CoercionError, QuillConfig};
pub use ignore::QuillIgnore;
pub use render::QuillInput;
pub use tree::FileTreeNode;
pub use types::{
    field_key, ui_key, CardSchema, FieldSchema, FieldType, UiContainerSchema, UiFieldSchema,
};

use std::collections::HashMap;
use std::sync::Arc;

use crate::value::QuillValue;

/// A quill format bundle.
#[derive(Clone)]
pub struct Quill {
    /// Quill-specific metadata
    pub metadata: HashMap<String, QuillValue>,
    /// Name of the quill
    pub name: String,
    /// Backend identifier (e.g., "typst")
    pub backend_id: String,
    /// Resolved backend, set by the engine after loading
    pub(crate) resolved_backend: Option<Arc<dyn crate::Backend>>,
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

impl std::fmt::Debug for Quill {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quill")
            .field("name", &self.name)
            .field("backend_id", &self.backend_id)
            .field(
                "resolved_backend",
                &self.resolved_backend.as_ref().map(|b| b.id()),
            )
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
