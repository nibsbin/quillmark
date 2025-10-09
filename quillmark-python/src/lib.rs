use pyo3::prelude::*;

mod enums;
mod errors;
mod types;

pub use enums::{PyOutputFormat, PySeverity};
pub use errors::{
    convert_render_error, CompilationError, ParseError, QuillmarkError, TemplateError,
};
pub use types::{
    PyArtifact, PyDiagnostic, PyLocation, PyParsedDocument, PyQuill, PyQuillmark, PyRenderResult,
    PyWorkflow,
};

#[pymodule]
fn _quillmark(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register classes
    m.add_class::<PyQuillmark>()?;
    m.add_class::<PyWorkflow>()?;
    m.add_class::<PyQuill>()?;
    m.add_class::<PyParsedDocument>()?;
    m.add_class::<PyRenderResult>()?;
    m.add_class::<PyArtifact>()?;
    m.add_class::<PyDiagnostic>()?;
    m.add_class::<PyLocation>()?;

    // Register enums
    m.add_class::<PyOutputFormat>()?;
    m.add_class::<PySeverity>()?;

    // Register exceptions
    m.add("QuillmarkError", m.py().get_type::<QuillmarkError>())?;
    m.add("ParseError", m.py().get_type::<ParseError>())?;
    m.add("TemplateError", m.py().get_type::<TemplateError>())?;
    m.add("CompilationError", m.py().get_type::<CompilationError>())?;

    Ok(())
}
