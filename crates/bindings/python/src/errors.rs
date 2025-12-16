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
    Python::attach(|py| match err {
        RenderError::InvalidFrontmatter { diag } => {
            let py_err = ParseError::new_err(diag.message.clone());
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::TemplateFailed { diag } => {
            let py_err = TemplateError::new_err(diag.message.clone());
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
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
        RenderError::DynamicAssetCollision { diag } => {
            let py_err = QuillmarkError::new_err(format!("Asset collision: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::DynamicFontCollision { diag } => {
            let py_err = QuillmarkError::new_err(format!("Font collision: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::EngineCreation { diag } => {
            let py_err =
                QuillmarkError::new_err(format!("Engine creation failed: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::FormatNotSupported { diag } => {
            let py_err = QuillmarkError::new_err(format!("Format not supported: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::UnsupportedBackend { diag } => {
            let py_err = QuillmarkError::new_err(format!("Unsupported backend: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::InputTooLarge { diag } => {
            let py_err = QuillmarkError::new_err(format!("Input too large: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::YamlTooLarge { diag } => {
            let py_err = QuillmarkError::new_err(format!("YAML too large: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::NestingTooDeep { diag } => {
            let py_err = QuillmarkError::new_err(format!("Nesting too deep: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::OutputTooLarge { diag } => {
            let py_err = QuillmarkError::new_err(format!("Output too large: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::ValidationFailed { diag } => {
            let py_err = QuillmarkError::new_err(format!("Validation failed: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::InvalidSchema { diag } => {
            let py_err = QuillmarkError::new_err(format!("Invalid schema: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
        RenderError::QuillConfig { diag } => {
            let py_err =
                QuillmarkError::new_err(format!("Quill configuration error: {}", diag.message));
            if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                let py_diag = crate::types::PyDiagnostic {
                    inner: (*diag).into(),
                };
                let _ = exc.setattr("diagnostic", py_diag);
            }
            py_err
        }
    })
}
