// Clean, non-duplicated imports
use pyo3::conversion::IntoPyObjectExt;
use pyo3::prelude::*; // PyResult, Python, etc.
use pyo3::pycell::PyRef; // PyRef
use pyo3::types::PyDict; // PyDict
use pyo3::{Bound, PyAny}; // Bound, PyAny

use quillmark::{
    Diagnostic, Location, OutputFormat, ParsedDocument, Quill, Quillmark, RenderResult, Workflow,
};
use std::collections::HashMap;
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

    fn register_quill(&mut self, quill: PyRef<PyQuill>) {
        self.inner.register_quill(quill.inner.clone());
    }

    fn workflow_from_quill_name(&self, name: &str) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .workflow_from_quill_name(name)
            .map_err(convert_render_error)?;
        Ok(PyWorkflow { inner: workflow })
    }

    fn workflow_from_quill(&self, quill: PyRef<PyQuill>) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .workflow_from_quill(&quill.inner)
            .map_err(convert_render_error)?;
        Ok(PyWorkflow { inner: workflow })
    }

    fn workflow_from_parsed(&self, parsed: PyRef<PyParsedDocument>) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .workflow_from_parsed(&parsed.inner)
            .map_err(convert_render_error)?;
        Ok(PyWorkflow { inner: workflow })
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
    fn render_source(
        &self,
        content: &str,
        format: Option<PyOutputFormat>,
    ) -> PyResult<PyRenderResult> {
        let rust_format = format.map(|f| f.into());
        let result = self
            .inner
            .render_source(content, rust_format)
            .map_err(convert_render_error)?;
        Ok(PyRenderResult { inner: result })
    }

    fn process_glue(&self, markdown: &str) -> PyResult<String> {
        self.inner
            .process_glue(markdown)
            .map_err(convert_render_error)
    }

    fn process_glue_parsed(&self, parsed: PyRef<PyParsedDocument>) -> PyResult<String> {
        self.inner
            .process_glue_parsed(&parsed.inner)
            .map_err(convert_render_error)
    }

    fn with_asset(&self, _filename: String, _contents: Vec<u8>) -> PyResult<()> {
        Err(PyErr::new::<crate::errors::QuillmarkError, _>(
            "Builder pattern methods (with_asset, with_font, etc.) are not yet supported in Python bindings. \
             Create a new workflow instead.",
        ))
    }

    fn with_assets(&self, _assets: HashMap<String, Vec<u8>>) -> PyResult<()> {
        Err(PyErr::new::<crate::errors::QuillmarkError, _>(
            "Builder pattern methods are not yet supported in Python bindings",
        ))
    }

    fn clear_assets(&self) -> PyResult<()> {
        Err(PyErr::new::<crate::errors::QuillmarkError, _>(
            "Builder pattern methods are not yet supported in Python bindings",
        ))
    }

    fn with_font(&self, _filename: String, _contents: Vec<u8>) -> PyResult<()> {
        Err(PyErr::new::<crate::errors::QuillmarkError, _>(
            "Builder pattern methods are not yet supported in Python bindings",
        ))
    }

    fn with_fonts(&self, _fonts: HashMap<String, Vec<u8>>) -> PyResult<()> {
        Err(PyErr::new::<crate::errors::QuillmarkError, _>(
            "Builder pattern methods are not yet supported in Python bindings",
        ))
    }

    fn clear_fonts(&self) -> PyResult<()> {
        Err(PyErr::new::<crate::errors::QuillmarkError, _>(
            "Builder pattern methods are not yet supported in Python bindings",
        ))
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

    fn dynamic_asset_names(&self) -> Vec<String> {
        self.inner.dynamic_asset_names()
    }

    fn dynamic_font_names(&self) -> Vec<String> {
        self.inner.dynamic_font_names()
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
    fn from_path(path: String) -> PyResult<Self> {
        let quill = Quill::from_path(PathBuf::from(path))
            .map_err(|e| PyErr::new::<crate::errors::QuillmarkError, _>(e.to_string()))?;
        Ok(PyQuill { inner: quill })
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
    fn glue_template(&self) -> &str {
        &self.inner.glue_template
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
    fn field_schemas<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        // Convert field_schemas to Python dict
        let dict = PyDict::new(py);
        for (key, schema) in &self.inner.field_schemas {
            // Convert FieldSchema to QuillValue, then to Python
            let quill_value = schema.to_quill_value();
            dict.set_item(key, quillvalue_to_py(py, &quill_value)?)?;
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
        let parsed = ParsedDocument::from_markdown(markdown)
            .map_err(|e| PyErr::new::<crate::errors::ParseError, _>(e.to_string()))?;
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

    fn quill_tag(&self) -> Option<&str> {
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
            .map(|d| PyDiagnostic { inner: d.clone() })
            .collect()
    }

    #[getter]
    fn output_format(&self) -> &str {
        match self.inner.output_format {
            OutputFormat::Pdf => "pdf",
            OutputFormat::Svg => "svg",
            OutputFormat::Txt => "txt",
        }
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
    pub(crate) inner: Diagnostic,
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
