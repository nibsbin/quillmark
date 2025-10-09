use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
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

    fn register_quill(&mut self, quill: &PyQuill) {
        self.inner.register_quill(quill.inner.clone());
    }

    fn workflow_from_quill_name(&self, name: &str) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .workflow_from_quill_name(name)
            .map_err(convert_render_error)?;
        Ok(PyWorkflow { inner: workflow })
    }

    fn workflow_from_quill(&self, quill: &PyQuill) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .workflow_from_quill(&quill.inner)
            .map_err(convert_render_error)?;
        Ok(PyWorkflow { inner: workflow })
    }

    fn workflow_from_parsed(&self, parsed: &PyParsedDocument) -> PyResult<PyWorkflow> {
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
        parsed: &PyParsedDocument,
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

    fn process_glue_parsed(&self, parsed: &PyParsedDocument) -> PyResult<String> {
        self.inner
            .process_glue_parsed(&parsed.inner)
            .map_err(convert_render_error)
    }

    // Note: Builder pattern methods are not fully supported in Python bindings
    // due to Workflow not implementing Clone. For now, these are placeholder methods.
    fn with_asset(&self, _filename: String, _contents: Vec<u8>) -> PyResult<()> {
        Err(PyErr::new::<crate::errors::QuillmarkError, _>(
            "Builder pattern methods (with_asset, with_font, etc.) are not yet supported in Python bindings. \
             Create a new workflow instead."
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

    fn backend_id(&self) -> &str {
        self.inner.backend_id()
    }

    fn supported_formats(&self) -> Vec<PyOutputFormat> {
        self.inner
            .supported_formats()
            .iter()
            .map(|f| (*f).into())
            .collect()
    }

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
    fn backend(&self) -> Option<String> {
        self.inner
            .metadata
            .get("backend")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    #[getter]
    fn glue_template(&self) -> &str {
        &self.inner.glue_template
    }

    #[getter]
    fn metadata(&self, py: Python) -> PyResult<PyObject> {
        // Convert serde_yaml::Value to Python dict
        let dict = PyDict::new_bound(py);
        for (key, value) in &self.inner.metadata {
            dict.set_item(key, yaml_value_to_py(py, value)?)?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn field_schemas(&self, py: Python) -> PyResult<PyObject> {
        // Convert field_schemas to Python dict
        let dict = PyDict::new_bound(py);
        for (key, value) in &self.inner.field_schemas {
            dict.set_item(key, yaml_value_to_py(py, value)?)?;
        }
        Ok(dict.into())
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

    fn get_field(&self, key: &str, py: Python) -> PyResult<Option<PyObject>> {
        match self.inner.get_field(key) {
            Some(value) => Ok(Some(yaml_value_to_py(py, value)?)),
            None => Ok(None),
        }
    }

    fn fields(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (key, value) in self.inner.fields() {
            dict.set_item(key, yaml_value_to_py(py, value)?)?;
        }
        Ok(dict.into())
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
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner)
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

// Helper function to convert YAML values to Python objects
fn yaml_value_to_py(py: Python, value: &serde_yaml::Value) -> PyResult<PyObject> {
    match value {
        serde_yaml::Value::Null => Ok(py.None()),
        serde_yaml::Value::Bool(b) => Ok(b.to_object(py)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(py.None())
            }
        }
        serde_yaml::Value::String(s) => Ok(s.to_object(py)),
        serde_yaml::Value::Sequence(seq) => {
            let list = pyo3::types::PyList::empty_bound(py);
            for item in seq {
                list.append(yaml_value_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        serde_yaml::Value::Mapping(map) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in map {
                if let serde_yaml::Value::String(key) = k {
                    dict.set_item(key, yaml_value_to_py(py, v)?)?;
                }
            }
            Ok(dict.into())
        }
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_py(py, &tagged.value),
    }
}
