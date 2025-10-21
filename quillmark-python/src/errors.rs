use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use quillmark_core::RenderError;

use crate::types::PyDiagnostic;

// Base exception
create_exception!(_quillmark, QuillmarkError, PyException);

// Specific exceptions
create_exception!(_quillmark, ParseError, QuillmarkError);
create_exception!(_quillmark, TemplateError, QuillmarkError);
create_exception!(_quillmark, CompilationError, QuillmarkError);

pub fn convert_render_error(err: RenderError) -> PyErr {
    match err {
        RenderError::InvalidFrontmatter { diag, .. } => {
            let message = diag.message.clone();
            let py_diag = PyDiagnostic { inner: diag };
            Python::with_gil(|py| {
                let exc = ParseError::new_err(message);
                if let Ok(value) = exc.value(py).downcast::<PyAny>() {
                    let _ = value.setattr("diagnostic", py_diag);
                }
                exc
            })
        }
        RenderError::TemplateFailed { diag, .. } => {
            let message = diag.message.clone();
            let py_diag = PyDiagnostic { inner: diag };
            Python::with_gil(|py| {
                let exc = TemplateError::new_err(message);
                if let Ok(value) = exc.value(py).downcast::<PyAny>() {
                    let _ = value.setattr("diagnostic", py_diag);
                }
                exc
            })
        }
        RenderError::CompilationFailed(count, diags) => {
            let message = format!("Compilation failed with {} error(s)", count);
            let py_diags: Vec<PyDiagnostic> = diags
                .into_iter()
                .map(|d| PyDiagnostic { inner: d })
                .collect();
            Python::with_gil(|py| {
                let exc = CompilationError::new_err(message);
                if let Ok(value) = exc.value(py).downcast::<PyAny>() {
                    let _ = value.setattr("diagnostics", py_diags);
                }
                exc
            })
        }
        RenderError::DynamicAssetCollision { filename, message } => {
            QuillmarkError::new_err(format!("Asset collision ({}): {}", filename, message))
        }
        RenderError::DynamicFontCollision { filename, message } => {
            QuillmarkError::new_err(format!("Font collision ({}): {}", filename, message))
        }
        RenderError::Other(msg) => QuillmarkError::new_err(msg.to_string()),
        RenderError::EngineCreation { diag, .. } => {
            let message = diag.message.clone();
            let py_diag = PyDiagnostic { inner: diag };
            Python::with_gil(|py| {
                let exc = QuillmarkError::new_err(message);
                if let Ok(value) = exc.value(py).downcast::<PyAny>() {
                    let _ = value.setattr("diagnostic", py_diag);
                }
                exc
            })
        }
        RenderError::FormatNotSupported { backend, format } => {
            QuillmarkError::new_err(format!("Format {:?} not supported by {}", format, backend))
        }
        RenderError::UnsupportedBackend(backend) => {
            QuillmarkError::new_err(format!("Unsupported backend: {}", backend))
        }
        RenderError::Internal(err) => QuillmarkError::new_err(format!("Internal error: {}", err)),
        RenderError::Template(err) => TemplateError::new_err(err.to_string()),
        _ => QuillmarkError::new_err(err.to_string()),
    }
}
