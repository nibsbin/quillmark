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
        RenderError::InvalidFrontmatter { diag } => ParseError::new_err(diag.message.clone()),
        RenderError::TemplateFailed { diag } => TemplateError::new_err(diag.message.clone()),
        RenderError::CompilationFailed { diags } => {
            CompilationError::new_err(format!("Compilation failed with {} error(s)", diags.len()))
        }
        RenderError::DynamicAssetCollision { diag } => {
            QuillmarkError::new_err(format!("Asset collision: {}", diag.message))
        }
        RenderError::DynamicFontCollision { diag } => {
            QuillmarkError::new_err(format!("Font collision: {}", diag.message))
        }
        RenderError::EngineCreation { diag } => {
            QuillmarkError::new_err(format!("Engine creation failed: {}", diag.message))
        }
        RenderError::FormatNotSupported { diag } => {
            QuillmarkError::new_err(format!("Format not supported: {}", diag.message))
        }
        RenderError::UnsupportedBackend { diag } => {
            QuillmarkError::new_err(format!("Unsupported backend: {}", diag.message))
        }
        RenderError::InputTooLarge { diag } => {
            QuillmarkError::new_err(format!("Input too large: {}", diag.message))
        }
        RenderError::YamlTooLarge { diag } => {
            QuillmarkError::new_err(format!("YAML too large: {}", diag.message))
        }
        RenderError::NestingTooDeep { diag } => {
            QuillmarkError::new_err(format!("Nesting too deep: {}", diag.message))
        }
        RenderError::OutputTooLarge { diag } => {
            QuillmarkError::new_err(format!("Output too large: {}", diag.message))
        }
    }
}
