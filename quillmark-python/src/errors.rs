use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use quillmark_core::RenderError;

// Base exception
create_exception!(_quillmark, QuillmarkError, PyException);

// Specific exceptions
create_exception!(_quillmark, ParseError, QuillmarkError);
create_exception!(_quillmark, TemplateError, QuillmarkError);
create_exception!(_quillmark, CompilationError, QuillmarkError);

pub fn convert_render_error(err: RenderError) -> PyErr {
    match err {
        RenderError::InvalidFrontmatter { diag, .. } => ParseError::new_err(diag.message.clone()),
        RenderError::TemplateFailed { diag, .. } => TemplateError::new_err(diag.message.clone()),
        RenderError::CompilationFailed(count, _diags) => {
            CompilationError::new_err(format!("Compilation failed with {} error(s)", count))
        }
        RenderError::DynamicAssetCollision { filename, message } => {
            QuillmarkError::new_err(format!("Asset collision ({}): {}", filename, message))
        }
        RenderError::DynamicFontCollision { filename, message } => {
            QuillmarkError::new_err(format!("Font collision ({}): {}", filename, message))
        }
        RenderError::Other(msg) => QuillmarkError::new_err(msg.to_string()),
        RenderError::EngineCreation { .. } => {
            QuillmarkError::new_err("Engine creation failed".to_string())
        }
        RenderError::FormatNotSupported { .. } => {
            QuillmarkError::new_err("Format not supported".to_string())
        }
        RenderError::UnsupportedBackend(backend) => {
            QuillmarkError::new_err(format!("Unsupported backend: {}", backend))
        }
        RenderError::Internal(err) => QuillmarkError::new_err(format!("Internal error: {}", err)),
        RenderError::Template(err) => TemplateError::new_err(err.to_string()),
        _ => QuillmarkError::new_err(err.to_string()),
    }
}
