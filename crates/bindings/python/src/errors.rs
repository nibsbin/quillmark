use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use quillmark_core::{Diagnostic, RenderError};

// Base exception
create_exception!(_quillmark, QuillmarkError, PyException);

// Specific exceptions
create_exception!(_quillmark, ParseError, QuillmarkError);
create_exception!(_quillmark, TemplateError, QuillmarkError);
create_exception!(_quillmark, CompilationError, QuillmarkError);

fn with_diag_attached(py: Python, py_err: PyErr, diag: Diagnostic) -> PyErr {
    if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
        let py_diag = crate::types::PyDiagnostic { inner: diag.into() };
        let _ = exc.setattr("diagnostic", py_diag);
    }
    py_err
}

pub fn convert_render_error(err: RenderError) -> PyErr {
    Python::attach(|py| match err {
        RenderError::CompilationFailed { diags } => {
            let py_err = CompilationError::new_err(format!(
                "Compilation failed with {} error(s)",
                diags.len()
            ));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diags: Vec<crate::types::PyDiagnostic> = diags
                    .into_iter()
                    .map(|d| crate::types::PyDiagnostic { inner: d.into() })
                    .collect();
                let _ = exc.setattr("diagnostics", py_diags);
            }
            py_err
        }
        RenderError::InvalidFrontmatter { diag } => {
            with_diag_attached(py, ParseError::new_err(diag.message.clone()), *diag)
        }
        RenderError::EngineCreation { diag }
        | RenderError::FormatNotSupported { diag }
        | RenderError::UnsupportedBackend { diag }
        | RenderError::DynamicAssetCollision { diag }
        | RenderError::DynamicFontCollision { diag }
        | RenderError::ValidationFailed { diag }
        | RenderError::QuillConfig { diag }
        | RenderError::NoBackend { diag } => {
            with_diag_attached(py, QuillmarkError::new_err(diag.message.clone()), *diag)
        }
    })
}
