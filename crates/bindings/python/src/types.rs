use pyo3::conversion::IntoPyObjectExt;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*; // PyResult, Python, etc.
use pyo3::pycell::PyRef; // PyRef
use pyo3::types::PyDict; // PyDict
use pyo3::Bound; // Bound

use quillmark::{
    Document, Location, OutputFormat, Quill, Quillmark, RenderOptions, RenderResult, RenderSession,
    SerializableDiagnostic, Workflow,
};
use std::path::PathBuf;

use crate::enums::{PyOutputFormat, PySeverity};
use crate::errors::{convert_edit_error, convert_render_error};

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

    fn quill_from_path(&self, path: PathBuf) -> PyResult<PyQuill> {
        let quill = self
            .inner
            .quill_from_path(&path)
            .map_err(convert_render_error)?;
        Ok(PyQuill { inner: quill })
    }

    fn workflow(&self, quill: PyRef<PyQuill>) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .workflow(&quill.inner)
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
}

// Workflow wrapper
#[pyclass(name = "Workflow")]
pub struct PyWorkflow {
    pub(crate) inner: Workflow,
}

#[pymethods]
impl PyWorkflow {
    #[pyo3(signature = (doc, format=None))]
    fn render(
        &self,
        doc: PyRef<PyDocument>,
        format: Option<PyOutputFormat>,
    ) -> PyResult<PyRenderResult> {
        let rust_format = format.map(|f| f.into());
        let mut result = self
            .inner
            .render(&doc.inner, rust_format)
            .map_err(convert_render_error)?;
        // Prepend parse-time warnings so both parse and render diagnostics
        // travel on the single RenderResult.warnings channel.
        let parse_warnings: Vec<_> = doc
            .parse_warnings
            .iter()
            .map(|d| d.clone_without_source())
            .collect();
        result.warnings.splice(0..0, parse_warnings);
        Ok(PyRenderResult { inner: result })
    }

    fn open(&self, doc: PyRef<PyDocument>) -> PyResult<PyRenderSession> {
        let session = self.inner.open(&doc.inner).map_err(convert_render_error)?;
        Ok(PyRenderSession { inner: session })
    }

    /// Perform a dry run validation without backend compilation.
    ///
    /// Raises QuillmarkError with diagnostic payload on validation failure.
    fn dry_run(&self, doc: PyRef<PyDocument>) -> PyResult<()> {
        self.inner.dry_run(&doc.inner).map_err(convert_render_error)
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
    fn quill_ref(&self) -> String {
        self.inner.quill_ref()
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
        &self.inner.backend_id
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
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.metadata {
            dict.set_item(key, quillvalue_to_py(py, value)?)?;
        }
        Ok(dict)
    }

    #[getter]
    fn schema<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let yaml = self
            .inner
            .config
            .public_schema_yaml()
            .map_err(|e| PyValueError::new_err(format!("schema: {}", e)))?;
        Ok(yaml.into_pyobject(py)?.into_any())
    }

    #[getter]
    fn defaults<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in self.inner.config.defaults() {
            dict.set_item(key, quillvalue_to_py(py, &value)?)?;
        }
        Ok(dict)
    }

    #[getter]
    fn examples<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, values) in self.inner.config.examples() {
            let py_list = pyo3::types::PyList::empty(py);
            for value in values {
                py_list.append(quillvalue_to_py(py, &value)?)?;
            }
            dict.set_item(key, py_list)?;
        }
        Ok(dict)
    }

    fn supported_formats(&self) -> PyResult<Vec<PyOutputFormat>> {
        let formats = match self.inner.backend_id.as_str() {
            "typst" => vec![
                PyOutputFormat::PDF,
                PyOutputFormat::SVG,
                PyOutputFormat::PNG,
            ],
            _ => vec![],
        };
        Ok(formats)
    }

    #[pyo3(signature = (doc, format=None))]
    fn render(
        &self,
        doc: PyRef<'_, PyDocument>,
        format: Option<PyOutputFormat>,
    ) -> PyResult<PyRenderResult> {
        let rust_format = format.map(OutputFormat::from);
        let opts = RenderOptions {
            output_format: rust_format,
            ppi: None,
            pages: None,
        };
        let mut result = self
            .inner
            .render(doc.inner.clone(), &opts)
            .map_err(convert_render_error)?;
        let parse_warnings: Vec<_> = doc
            .parse_warnings
            .iter()
            .map(|d| d.clone_without_source())
            .collect();
        result.warnings.splice(0..0, parse_warnings);
        Ok(PyRenderResult { inner: result })
    }

    fn open(&self, doc: PyRef<'_, PyDocument>) -> PyResult<PyRenderSession> {
        let session = self
            .inner
            .open(doc.inner.clone())
            .map_err(convert_render_error)?;
        Ok(PyRenderSession { inner: session })
    }

    /// Project a document through this quill's schema.
    ///
    /// Returns a dict with keys `main`, `cards`, and `diagnostics`:
    ///
    /// - `main`: dict with `schema` (dict) and `values` (dict of field → value info)
    /// - `cards`: list of dicts in the same shape as `main`
    /// - `diagnostics`: list of dicts with `severity`, `code`, `message`, etc.
    ///
    /// Each `values` entry is a dict with:
    /// - `value`: the current document value, or `None` if absent
    /// - `default`: the schema default value, or `None` if none declared
    /// - `source`: one of `"document"`, `"default"`, or `"missing"`
    ///
    /// This is a **read-only snapshot**. Call `project_form` again after any
    /// edits to the document to obtain an updated projection.
    ///
    /// Cards with unknown tags are excluded from `cards`; each produces a
    /// diagnostic with code `"form::unknown_card_tag"`.
    fn project_form<'py>(
        &self,
        py: Python<'py>,
        doc: PyRef<'_, PyDocument>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let projection = quillmark::form::project_form(&self.inner, &doc.inner);

        // Serialise through serde_json → Python dict to avoid writing bespoke
        // conversion for every nested type (CardSchema, FormFieldValue, etc.).
        let json_value = serde_json::to_value(&projection).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "project_form: serialization failed: {e}"
            ))
        })?;
        let py_obj = json_to_py(py, &json_value)?;
        let dict = py_obj.downcast::<PyDict>().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "project_form: expected object at top level",
            )
        })?;
        Ok(dict.clone())
    }
}

/// Python wrapper for the typed Quillmark `Document`.
///
/// Exposes:
/// - `from_markdown(markdown)` — static constructor
/// - `to_markdown()` — emit canonical Quillmark Markdown
/// - `quill_ref()` — quill reference string
/// - `frontmatter` — dict of typed YAML fields (no QUILL/BODY/CARDS)
/// - `body` — global Markdown body (str, never None)
/// - `cards` — list of `Card` dicts
/// - `warnings` — list of `Diagnostic` objects
#[pyclass(name = "Document")]
pub struct PyDocument {
    pub(crate) inner: Document,
    pub(crate) parse_warnings: Vec<quillmark_core::Diagnostic>,
}

#[pymethods]
impl PyDocument {
    #[staticmethod]
    fn from_markdown(markdown: &str) -> PyResult<Self> {
        let output = Document::from_markdown_with_warnings(markdown).map_err(|e| {
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
        Ok(PyDocument {
            inner: output.document,
            parse_warnings: output.warnings,
        })
    }

    /// Emit canonical Quillmark Markdown.
    ///
    /// Returns the document serialised as a Quillmark Markdown string.
    /// The output is type-fidelity round-trip safe: re-parsing the result
    /// produces a `Document` equal to `self` by value and by type.
    fn to_markdown(&self) -> String {
        self.inner.to_markdown()
    }

    /// The QUILL reference string (e.g. `"usaf_memo@0.1"`).
    fn quill_ref(&self) -> String {
        self.inner.quill_reference().to_string()
    }

    /// Non-fatal parse-time warnings.
    #[getter]
    fn warnings(&self) -> Vec<PyDiagnostic> {
        self.parse_warnings
            .iter()
            .map(|d| PyDiagnostic { inner: d.into() })
            .collect()
    }

    /// Global Markdown body (str, never None).
    #[getter]
    fn body(&self) -> &str {
        self.inner.body()
    }

    /// Typed YAML frontmatter fields (no QUILL, BODY, or CARDS keys).
    #[getter]
    fn frontmatter<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in self.inner.frontmatter() {
            dict.set_item(key, quillvalue_to_py(py, value)?)?;
        }
        Ok(dict)
    }

    /// Ordered list of card blocks.
    ///
    /// Each card is a dict with keys: `tag` (str), `fields` (dict), `body` (str).
    #[getter]
    fn cards<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        let mut result = Vec::new();
        for card in self.inner.cards() {
            let d = PyDict::new(py);
            d.set_item("tag", card.tag())?;
            let fields_dict = PyDict::new(py);
            for (k, v) in card.fields() {
                fields_dict.set_item(k, quillvalue_to_py(py, v)?)?;
            }
            d.set_item("fields", fields_dict)?;
            d.set_item("body", card.body())?;
            result.push(d);
        }
        Ok(result)
    }

    // ── Mutators ──────────────────────────────────────────────────────────────

    /// Set a frontmatter field by name.
    ///
    /// Raises `quillmark.EditError` if `name` is a reserved sentinel
    /// (`BODY`, `CARDS`, `QUILL`, `CARD`) or does not match `[a-z_][a-z0-9_]*`.
    ///
    /// This method never modifies `warnings`.
    fn set_field(&mut self, name: &str, value: Bound<'_, PyAny>) -> PyResult<()> {
        let qv = py_to_quillvalue(&value)?;
        self.inner.set_field(name, qv).map_err(convert_edit_error)
    }

    /// Remove a frontmatter field by name, returning the value or `None`.
    ///
    /// This method never modifies `warnings`.
    fn remove_field<'py>(&mut self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyAny>> {
        match self.inner.remove_field(name) {
            Some(v) => quillvalue_to_py(py, &v),
            None => py.None().into_bound_py_any(py),
        }
    }

    /// Replace the QUILL reference string.
    ///
    /// Raises `ValueError` if `ref_str` is not a valid `QuillReference`.
    ///
    /// This method never modifies `warnings`.
    fn set_quill_ref(&mut self, ref_str: &str) -> PyResult<()> {
        let qr: quillmark_core::QuillReference = ref_str.parse().map_err(|e| {
            PyValueError::new_err(format!("invalid QuillReference '{}': {}", ref_str, e))
        })?;
        self.inner.set_quill_ref(qr);
        Ok(())
    }

    /// Replace the global Markdown body.
    ///
    /// This method never modifies `warnings`.
    fn replace_body(&mut self, body: &str) {
        self.inner.replace_body(body);
    }

    /// Append a card to the card list.
    ///
    /// `card` must be a dict with a `tag` key (str) and optional `fields` (dict)
    /// and `body` (str).
    ///
    /// Raises `quillmark.EditError` if `card["tag"]` is not a valid tag name or
    /// if any field name is invalid.
    ///
    /// This method never modifies `warnings`.
    fn push_card(&mut self, card: Bound<'_, PyAny>) -> PyResult<()> {
        let core_card = py_dict_to_card(&card)?;
        self.inner.push_card(core_card).map_err(convert_edit_error)
    }

    /// Insert a card at the given index.
    ///
    /// `index` must be in `0..=len`. Out-of-range raises `quillmark.EditError`.
    ///
    /// This method never modifies `warnings`.
    fn insert_card(&mut self, index: usize, card: Bound<'_, PyAny>) -> PyResult<()> {
        let core_card = py_dict_to_card(&card)?;
        self.inner
            .insert_card(index, core_card)
            .map_err(convert_edit_error)
    }

    /// Remove and return the card at `index`, or `None` if out of range.
    ///
    /// This method never modifies `warnings`.
    fn remove_card<'py>(
        &mut self,
        py: Python<'py>,
        index: usize,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        match self.inner.remove_card(index) {
            Some(card) => {
                let d = PyDict::new(py);
                d.set_item("tag", card.tag())?;
                let fields_dict = PyDict::new(py);
                for (k, v) in card.fields() {
                    fields_dict.set_item(k, quillvalue_to_py(py, v)?)?;
                }
                d.set_item("fields", fields_dict)?;
                d.set_item("body", card.body())?;
                Ok(Some(d))
            }
            None => Ok(None),
        }
    }

    /// Move the card at `from_idx` to position `to_idx`.
    ///
    /// `from_idx == to_idx` is a no-op. Both indices must be in `0..len`.
    /// Out-of-range raises `quillmark.EditError`.
    ///
    /// This method never modifies `warnings`.
    fn move_card(&mut self, from_idx: usize, to_idx: usize) -> PyResult<()> {
        self.inner
            .move_card(from_idx, to_idx)
            .map_err(convert_edit_error)
    }

    /// Update a field on the card at `index`.
    ///
    /// Raises `quillmark.EditError` if `index` is out of range, `name` is
    /// reserved or invalid, or `value` cannot be converted.
    ///
    /// This method never modifies `warnings`.
    fn update_card_field(
        &mut self,
        index: usize,
        name: &str,
        value: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let qv = py_to_quillvalue(&value)?;
        let len = self.inner.cards().len();
        let card = self.inner.card_mut(index).ok_or_else(|| {
            convert_edit_error(quillmark_core::EditError::IndexOutOfRange { index, len })
        })?;
        card.set_field(name, qv).map_err(convert_edit_error)
    }

    /// Replace the body of the card at `index`.
    ///
    /// Raises `quillmark.EditError` if `index` is out of range.
    ///
    /// This method never modifies `warnings`.
    fn update_card_body(&mut self, index: usize, body: &str) -> PyResult<()> {
        let len = self.inner.cards().len();
        let card = self.inner.card_mut(index).ok_or_else(|| {
            convert_edit_error(quillmark_core::EditError::IndexOutOfRange { index, len })
        })?;
        card.set_body(body);
        Ok(())
    }
}

// RenderResult wrapper
#[pyclass(name = "RenderResult")]
pub struct PyRenderResult {
    pub(crate) inner: RenderResult,
}

#[pyclass(name = "RenderSession")]
pub struct PyRenderSession {
    pub(crate) inner: RenderSession,
}

#[pymethods]
impl PyRenderSession {
    #[getter]
    fn page_count(&self) -> usize {
        self.inner.page_count()
    }

    #[pyo3(signature = (format=None, pages=None))]
    fn render(
        &self,
        format: Option<PyOutputFormat>,
        pages: Option<Vec<usize>>,
    ) -> PyResult<PyRenderResult> {
        let opts = RenderOptions {
            output_format: format.map(OutputFormat::from),
            ppi: None,
            pages,
        };
        let result = self.inner.render(&opts).map_err(convert_render_error)?;
        Ok(PyRenderResult { inner: result })
    }
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

    #[getter]
    fn output_format(&self) -> PyOutputFormat {
        self.inner.output_format.into()
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

    #[getter]
    fn mime_type(&self) -> &'static str {
        match self.output_format {
            OutputFormat::Pdf => "application/pdf",
            OutputFormat::Svg => "image/svg+xml",
            OutputFormat::Txt => "text/plain",
            OutputFormat::Png => "image/png",
        }
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

// ── Python → Rust conversion helpers ─────────────────────────────────────────

/// Convert a Python object to a [`quillmark_core::QuillValue`].
///
/// Supports: `None` → null, `bool`, `int`, `float`, `str`, `list`, `dict`.
fn py_to_quillvalue(value: &Bound<'_, PyAny>) -> PyResult<quillmark_core::QuillValue> {
    let json = py_to_json(value)?;
    Ok(quillmark_core::QuillValue::from_json(json))
}

fn py_to_json(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    use pyo3::types::{PyBool, PyFloat, PyInt, PyList, PyString};

    if value.is_none() {
        return Ok(serde_json::Value::Null);
    }
    if value.is_instance_of::<PyBool>() {
        let b: bool = value.extract()?;
        return Ok(serde_json::Value::Bool(b));
    }
    if value.is_instance_of::<PyInt>() {
        let i: i64 = value.extract()?;
        return Ok(serde_json::json!(i));
    }
    if value.is_instance_of::<PyFloat>() {
        let f: f64 = value.extract()?;
        return Ok(serde_json::json!(f));
    }
    if value.is_instance_of::<PyString>() {
        let s: String = value.extract()?;
        return Ok(serde_json::Value::String(s));
    }
    if value.is_instance_of::<PyList>() {
        let list = value.downcast::<PyList>()?;
        let arr: PyResult<Vec<serde_json::Value>> =
            list.iter().map(|item| py_to_json(&item)).collect();
        return Ok(serde_json::Value::Array(arr?));
    }
    if value.is_instance_of::<PyDict>() {
        let dict = value.downcast::<PyDict>()?;
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, py_to_json(&v)?);
        }
        return Ok(serde_json::Value::Object(map));
    }
    // Fallback: convert to string
    let s = value.str()?.to_string();
    Ok(serde_json::Value::String(s))
}

/// Convert a Python dict `{"tag": str, "fields"?: dict, "body"?: str}` to a
/// [`quillmark_core::Card`].  Raises `EditError` on invalid tag or field names.
fn py_dict_to_card(value: &Bound<'_, PyAny>) -> PyResult<quillmark_core::Card> {
    let dict = value
        .downcast::<PyDict>()
        .map_err(|_| PyValueError::new_err("card must be a dict with a 'tag' key"))?;

    let tag: String = dict
        .get_item("tag")?
        .ok_or_else(|| PyValueError::new_err("card dict must have a 'tag' key"))?
        .extract()?;

    let mut card = quillmark_core::Card::new(tag).map_err(convert_edit_error)?;

    if let Some(fields_val) = dict.get_item("fields")? {
        let fields_dict = fields_val
            .downcast::<PyDict>()
            .map_err(|_| PyValueError::new_err("card 'fields' must be a dict"))?;
        for (k, v) in fields_dict.iter() {
            let field_name: String = k.extract()?;
            let qv = py_to_quillvalue(&v)?;
            card.set_field(&field_name, qv)
                .map_err(convert_edit_error)?;
        }
    }

    if let Some(body_val) = dict.get_item("body")? {
        let body: String = body_val.extract()?;
        card.set_body(body);
    }

    Ok(card)
}
