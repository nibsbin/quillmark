pub mod blueprint;
pub mod info;
pub mod render;
pub mod schema;
pub mod validate;

use crate::errors::{CliError, Result};
use quillmark::Quill;
use std::path::Path;

/// [`quillmark::quill_from_path`] with a clearer missing-directory message.
pub fn load_quill(path: &Path) -> Result<Quill> {
    if !path.exists() {
        return Err(CliError::InvalidArgument(format!(
            "Quill directory not found: {}",
            path.display()
        )));
    }
    Ok(quillmark::quill_from_path(path)?)
}
