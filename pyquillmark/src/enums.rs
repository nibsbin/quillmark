use pyo3::prelude::*;
use quillmark_core::{OutputFormat, Severity};

#[pyclass(name = "OutputFormat", eq, eq_int)]
#[derive(Clone, Copy, PartialEq)]
pub enum PyOutputFormat {
    PDF,
    SVG,
    TXT,
}

impl From<PyOutputFormat> for OutputFormat {
    fn from(val: PyOutputFormat) -> Self {
        match val {
            PyOutputFormat::PDF => OutputFormat::Pdf,
            PyOutputFormat::SVG => OutputFormat::Svg,
            PyOutputFormat::TXT => OutputFormat::Txt,
        }
    }
}

impl From<OutputFormat> for PyOutputFormat {
    fn from(val: OutputFormat) -> Self {
        match val {
            OutputFormat::Pdf => PyOutputFormat::PDF,
            OutputFormat::Svg => PyOutputFormat::SVG,
            OutputFormat::Txt => PyOutputFormat::TXT,
        }
    }
}

#[pyclass(name = "Severity", eq, eq_int)]
#[derive(Clone, Copy, PartialEq)]
pub enum PySeverity {
    ERROR,
    WARNING,
    NOTE,
}

impl From<PySeverity> for Severity {
    fn from(val: PySeverity) -> Self {
        match val {
            PySeverity::ERROR => Severity::Error,
            PySeverity::WARNING => Severity::Warning,
            PySeverity::NOTE => Severity::Note,
        }
    }
}

impl From<Severity> for PySeverity {
    fn from(val: Severity) -> Self {
        match val {
            Severity::Error => PySeverity::ERROR,
            Severity::Warning => PySeverity::WARNING,
            Severity::Note => PySeverity::NOTE,
        }
    }
}
