// Clean, non-duplicated imports
use pyo3::conversion::IntoPyObjectExt;
use pyo3::prelude::*; // PyResult, Python, etc.
use pyo3::pycell::PyRef; // PyRef
use pyo3::types::PyDict; // PyDict
use pyo3::{Bound, PyAny}; // Bound, PyAny

use quillmark::{
    Location, OutputFormat, ParsedDocument, Quill, Quillmark, RenderResult, SerializableDiagnostic,
    Workflow,
};
use std::path::PathBuf;

use crate::enums::{PyOutputFormat, PySeverity};
use crate::errors::convert_render_error;

// Quillmark Engine wrapper
#[pyclass(name = "Quillmark")]
pub struct PyQuillmark {
    inner: Quillmark,
}

#[pymethods]
impl PyQuillmark {
    #[new]
    fn new() -> Self {
        Self {
            inner: Quillmark::new(),
        }
    }

    fn register_quill(&mut self, quill: PyRef<PyQuill>) -> PyResult<()> {
        self.inner
            .register_quill(quill.inner.clone())
            .map_err(convert_render_error)?;
        Ok(())
    }

    fn workflow(&self, quill_ref: &Bound<'_, PyAny>) -> PyResult<PyWorkflow> {
        // Handle string (quill name)
        if let Ok(name) = quill_ref.extract::<String>() {
            let workflow = self
                .inner
                .workflow(name.as_str())
                .map_err(convert_render_error)?;
            return Ok(PyWorkflow { inner: workflow });
        }

        // Handle PyQuill
        if let Ok(quill) = quill_ref.extract::<PyRef<PyQuill>>() {
            let workflow = self
                .inner
                .workflow(&quill.inner)
                .map_err(convert_render_error)?;
            return Ok(PyWorkflow { inner: workflow });
        }

        // Handle PyParsedDocument
        if let Ok(parsed) = quill_ref.extract::<PyRef<PyParsedDocument>>() {
            let workflow = self
                .inner
                .workflow(&parsed.inner)
                .map_err(convert_render_error)?;
            return Ok(PyWorkflow { inner: workflow });
        }

        Err(pyo3::exceptions::PyTypeError::new_err(
            "workflow() expects a string (quill name), Quill object, or ParsedDocument",
        ))
    }

    fn registered_backends(&self) -> Vec<String> {
        self.inner
            .registered_backends()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn registered_quills(&self) -> Vec<String> {
        self.inner
            .registered_quills()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

// Workflow wrapper
#[pyclass(name = "Workflow")]
pub struct PyWorkflow {
    pub(crate) inner: Workflow,
}

#[pymethods]
impl PyWorkflow {
    #[pyo3(signature = (parsed, format=None))]
    fn render(
        &self,
        parsed: PyRef<PyParsedDocument>,
        format: Option<PyOutputFormat>,
    ) -> PyResult<PyRenderResult> {
        let rust_format = format.map(|f| f.into());
        let result = self
            .inner
            .render(&parsed.inner, rust_format)
            .map_err(convert_render_error)?;
        Ok(PyRenderResult { inner: result })
    }

    #[pyo3(signature = (content, format=None))]
    fn render_plated(
        &self,
        content: &str,
        format: Option<PyOutputFormat>,
    ) -> PyResult<PyRenderResult> {
        let rust_format = format.map(|f| f.into());
        let result = self
            .inner
            .render_plate(content, rust_format)
            .map_err(convert_render_error)?;
        Ok(PyRenderResult { inner: result })
    }

    fn process_plate(&self, parsed: PyRef<PyParsedDocument>) -> PyResult<String> {
        self.inner
            .process_plate(&parsed.inner)
            .map_err(convert_render_error)
    }

    /// Perform a dry run validation without backend compilation.
    ///
    /// Raises QuillmarkError with diagnostic payload on validation failure.
    fn dry_run(&self, parsed: PyRef<PyParsedDocument>) -> PyResult<()> {
        self.inner
            .dry_run(&parsed.inner)
            .map_err(convert_render_error)
    }

    #[getter]
    fn backend_id(&self) -> &str {
        self.inner.backend_id()
    }

    #[getter]
    fn supported_formats(&self) -> Vec<PyOutputFormat> {
        self.inner
            .supported_formats()
            .iter()
            .map(|f| (*f).into())
            .collect()
    }

    #[getter]
    fn quill_name(&self) -> &str {
        self.inner.quill_name()
    }
}

// Quill wrapper
#[pyclass(name = "Quill")]
#[derive(Clone)]
pub struct PyQuill {
    pub(crate) inner: Quill,
}

#[pymethods]
impl PyQuill {
    #[staticmethod]
    fn from_path(path: PathBuf) -> PyResult<Self> {
        let quill = Quill::from_path(path)
            .map_err(|e| PyErr::new::<crate::errors::QuillmarkError, _>(e.to_string()))?;
        Ok(PyQuill { inner: quill })
    }

    #[getter]
    fn print_tree(&self) -> String {
        self.inner.files.print_tree().clone()
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn backend(&self) -> &str {
        self.inner
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
    }

    #[getter]
    fn plate(&self) -> Option<String> {
        self.inner.plate.clone()
    }

    #[getter]
    fn example(&self) -> Option<String> {
        self.inner.example.clone()
    }

    #[getter]
    fn metadata<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        // Convert QuillValue to Python dict
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.metadata {
            dict.set_item(key, quillvalue_to_py(py, value)?)?;
        }
        Ok(dict)
    }

    #[getter]
    fn schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        // Convert serde_json::Value to Python object
        json_to_py(py, &self.inner.schema)
    }

    #[getter]
    fn defaults<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        // Convert cached defaults HashMap to Python dict
        let dict = PyDict::new(py);
        for (key, value) in self.inner.extract_defaults() {
            dict.set_item(key, quillvalue_to_py(py, value)?)?;
        }
        Ok(dict)
    }

    #[getter]
    fn examples<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        // Convert cached examples HashMap to Python dict of lists
        let dict = PyDict::new(py);
        for (key, values) in self.inner.extract_examples() {
            let py_list = pyo3::types::PyList::empty(py);
            for value in values {
                py_list.append(quillvalue_to_py(py, value)?)?;
            }
            dict.set_item(key, py_list)?;
        }
        Ok(dict)
    }

    fn supported_formats(&self) -> PyResult<Vec<PyOutputFormat>> {
        // Get backend from metadata
        let backend = self
            .inner
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PyErr::new::<crate::errors::QuillmarkError, _>(
                    "Quill metadata missing 'backend' field",
                )
            })?;

        // Determine supported formats based on backend
        let formats = match backend {
            "typst" => vec![PyOutputFormat::PDF, PyOutputFormat::SVG],
            "acroform" => vec![PyOutputFormat::PDF],
            _ => vec![],
        };

        Ok(formats)
    }
}

// ParsedDocument wrapper
#[pyclass(name = "ParsedDocument")]
pub struct PyParsedDocument {
    pub(crate) inner: ParsedDocument,
}

#[pymethods]
impl PyParsedDocument {
    #[staticmethod]
    fn from_markdown(markdown: &str) -> PyResult<Self> {
        let parsed = ParsedDocument::from_markdown(markdown).map_err(|e| {
            let py_err = PyErr::new::<crate::errors::ParseError, _>(e.to_string());
            Python::attach(|py| {
                if let Ok(exc) = py_err.value(py).downcast::<pyo3::types::PyAny>() {
                    let diag = e.to_diagnostic();
                    let py_diag = crate::types::PyDiagnostic { inner: diag.into() };
                    let _ = exc.setattr("diagnostic", py_diag);
                }
            });
            py_err
        })?;
        Ok(PyParsedDocument { inner: parsed })
    }

    fn body(&self) -> Option<&str> {
        self.inner.body()
    }

    fn get_field<'py>(&self, key: &str, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        match self.inner.get_field(key) {
            Some(value) => Ok(Some(quillvalue_to_py(py, value)?)),
            None => Ok(None),
        }
    }

    #[getter]
    fn fields<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in self.inner.fields() {
            dict.set_item(key, quillvalue_to_py(py, value)?)?;
        }
        Ok(dict)
    }

    fn quill_tag(&self) -> &str {
        self.inner.quill_tag()
    }
}

// RenderResult wrapper
#[pyclass(name = "RenderResult")]
pub struct PyRenderResult {
    pub(crate) inner: RenderResult,
}

#[pymethods]
impl PyRenderResult {
    #[getter]
    fn artifacts(&self) -> Vec<PyArtifact> {
        self.inner
            .artifacts
            .iter()
            .map(|a| PyArtifact {
                inner: a.bytes.clone(),
                output_format: a.output_format,
            })
            .collect()
    }

    #[getter]
    fn warnings(&self) -> Vec<PyDiagnostic> {
        self.inner
            .warnings
            .iter()
            .map(|d| PyDiagnostic { inner: d.into() })
            .collect()
    }
}

// Artifact wrapper
#[pyclass(name = "Artifact")]
#[derive(Clone)]
pub struct PyArtifact {
    pub(crate) inner: Vec<u8>,
    pub(crate) output_format: OutputFormat,
}

#[pymethods]
impl PyArtifact {
    #[getter]
    fn bytes(&self) -> Vec<u8> {
        self.inner.clone()
    }

    #[getter]
    fn output_format(&self) -> PyOutputFormat {
        self.output_format.into()
    }

    fn save(&self, path: String) -> PyResult<()> {
        std::fs::write(&path, &self.inner).map_err(|e| {
            PyErr::new::<crate::errors::QuillmarkError, _>(format!(
                "Failed to save artifact to {}: {}",
                path, e
            ))
        })
    }
}

// Diagnostic wrapper
#[pyclass(name = "Diagnostic")]
#[derive(Clone)]
pub struct PyDiagnostic {
    pub(crate) inner: SerializableDiagnostic,
}

#[pymethods]
impl PyDiagnostic {
    #[getter]
    fn severity(&self) -> PySeverity {
        self.inner.severity.into()
    }

    #[getter]
    fn message(&self) -> &str {
        &self.inner.message
    }

    #[getter]
    fn code(&self) -> Option<&str> {
        self.inner.code.as_deref()
    }

    #[getter]
    fn primary(&self) -> Option<PyLocation> {
        self.inner
            .primary
            .as_ref()
            .map(|l| PyLocation { inner: l.clone() })
    }

    #[getter]
    fn hint(&self) -> Option<&str> {
        self.inner.hint.as_deref()
    }

    #[getter]
    fn source_chain(&self) -> Vec<String> {
        self.inner.source_chain.clone()
    }
}

// Location wrapper
#[pyclass(name = "Location")]
#[derive(Clone)]
pub struct PyLocation {
    pub(crate) inner: Location,
}

#[pymethods]
impl PyLocation {
    #[getter]
    fn file(&self) -> &str {
        &self.inner.file
    }

    #[getter]
    fn line(&self) -> usize {
        self.inner.line as usize
    }

    #[getter]
    fn col(&self) -> usize {
        self.inner.col as usize
    }
}

// Helper function to convert QuillValue (backed by JSON) to Python objects
fn quillvalue_to_py<'py>(
    py: Python<'py>,
    value: &quillmark_core::QuillValue,
) -> PyResult<Bound<'py, PyAny>> {
    json_to_py(py, value.as_json())
}

// Helper function to convert JSON values to Python objects
fn json_to_py<'py>(py: Python<'py>, value: &serde_json::Value) -> PyResult<Bound<'py, PyAny>> {
    match value {
        serde_json::Value::Null => py.None().into_bound_py_any(py),
        serde_json::Value::Bool(b) => b.into_bound_py_any(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_bound_py_any(py)
            } else if let Some(u) = n.as_u64() {
                u.into_bound_py_any(py)
            } else if let Some(f) = n.as_f64() {
                f.into_bound_py_any(py)
            } else {
                py.None().into_bound_py_any(py)
            }
        }
        serde_json::Value::String(s) => s.as_str().into_bound_py_any(py),
        serde_json::Value::Array(arr) => {
            let list = pyo3::types::PyList::empty(py);
            for item in arr {
                let val = json_to_py(py, item)?;
                list.append(val)?;
            }
            Ok(list.into_any())
        }
        serde_json::Value::Object(map) => {
            let dict = pyo3::types::PyDict::new(py);
            for (key, val) in map {
                let py_val = json_to_py(py, val)?;
                dict.set_item(key, py_val)?;
            }
            Ok(dict.into_any())
        }
    }
}
