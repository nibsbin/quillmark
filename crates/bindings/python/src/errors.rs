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

fn attach_diagnostic(py: Python, py_err: &PyErr, diag: Diagnostic) {
    if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
        let py_diag = crate::types::PyDiagnostic { inner: diag.into() };
        let _ = exc.setattr("diagnostic", py_diag);
    }
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
        RenderError::Single { diag } => {
            let message = diag.message.clone();
            let py_err = match diag.code.as_deref() {
                Some(code) if code.starts_with("parse::") => ParseError::new_err(message),
                Some(code) if code.starts_with("template::") => TemplateError::new_err(message),
                _ => QuillmarkError::new_err(message),
            };
            attach_diagnostic(py, &py_err, *diag);
            py_err
        }
    })
}
