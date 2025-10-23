"""Quillmark - Python bindings for Quillmark."""

from ._quillmark import (
    Artifact,
    CompilationError,
    Diagnostic,
    Location,
    OutputFormat,  # No underscore prefix!
    ParseError,
    ParsedDocument,
    Quill,
    Quillmark,
    QuillmarkError,
    RenderResult,
    Severity,  # No underscore prefix!
    TemplateError,
    Workflow,
)

__all__ = [
    "Artifact",
    "CompilationError",
    "Diagnostic",
    "Location",
    "OutputFormat",
    "ParseError",
    "ParsedDocument",
    "Quill",
    "Quillmark",
    "QuillmarkError",
    "RenderResult",
    "Severity",
    "TemplateError",
    "Workflow",
]

__version__ = "0.1.0"