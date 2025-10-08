# Python Package Design for Quillmark

> **Status**: Design Phase  
> **Package Name**: `quillmark`  
> **Target**: Python 3.10+  

## Executive Summary

This document outlines the design for `quillmark`, a Python package that exposes the Quillmark rendering engine to Python applications. The package uses PyO3 for Rust-Python bindings and maturin for building and distributing binary wheels to PyPI.

**Design Goals:**
- Mirror the public API of the `quillmark` Rust crate
- Provide Pythonic interfaces following Python conventions
- Minimize overhead through efficient PyO3 bindings
- Support cross-platform distribution via PyPI wheels
- Use modern Python tooling (uv, maturin, ruff)

**Non-Goals:**
- Exposing low-level `quillmark-core` internals
- Supporting custom backend implementations in Python
- Async/streaming APIs (v1.0)

---

## Table of Contents

1. [Architecture](#architecture)
2. [Package Structure](#package-structure)
3. [Python API Design](#python-api-design)
4. [PyO3 Bindings Implementation](#pyo3-bindings-implementation)
5. [Build Configuration](#build-configuration)
6. [Development Workflow](#development-workflow)
7. [Testing Strategy](#testing-strategy)
8. [Distribution & Packaging](#distribution--packaging)
9. [Documentation](#documentation)
10. [Implementation Roadmap](#implementation-roadmap)

---

## Architecture

### Design Principles

1. **API Mirroring**: The Python API closely mirrors the Rust `quillmark` crate's public API
2. **Pythonic Idioms**: Use Python naming conventions, error handling, and patterns
3. **Zero-Copy**: Leverage PyO3's efficient memory management where possible
4. **Type Safety**: Provide comprehensive type hints for all public APIs
5. **Error Context**: Map Rust errors to Python exceptions with rich diagnostic information

### Component Diagram

```
┌─────────────────────────────────────────┐
│         Python Application              │
└──────────────┬──────────────────────────┘
               │ import quillmark
               ▼
┌─────────────────────────────────────────┐
│       quillmark (Python Layer)        │
│  - Type hints & stubs                   │
│  - Python-friendly wrappers             │
│  - Exception hierarchy                  │
└──────────────┬──────────────────────────┘
               │ PyO3 FFI
               ▼
┌─────────────────────────────────────────┐
│    _quillmark (Native Extension)      │
│  - PyO3 #[pyclass] wrappers             │
│  - Rust→Python type conversions         │
│  - Error mapping                        │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│         quillmark (Rust Crate)          │
│  - Quillmark engine                     │
│  - Workflow orchestration               │
│  - Backend compilation                  │
└─────────────────────────────────────────┘
```

---

## Package Structure

```
quillmark/
├── src/
│   ├── lib.rs              # PyO3 module entry point
│   ├── engine.rs           # Quillmark engine wrapper
│   ├── workflow.rs         # Workflow wrapper
│   ├── quill.rs            # Quill wrapper
│   ├── types.rs            # Output types (RenderResult, Artifact)
│   ├── enums.rs            # Enums (OutputFormat, Severity)
│   ├── errors.rs           # Exception definitions
│   └── conversions.rs      # Rust↔Python conversions
├── python/
│   └── quillmark/
│       ├── __init__.py     # Public API exports
│       ├── __init__.pyi    # Type stubs
│       └── py.typed        # PEP 561 marker
├── tests/
│   ├── test_engine.py
│   ├── test_workflow.py
│   ├── test_quill.py
│   ├── test_render.py
│   └── fixtures/           # Test quills
├── examples/
│   ├── basic.py
│   ├── dynamic_assets.py
│   └── batch.py
├── Cargo.toml              # Rust dependencies
├── pyproject.toml          # Python project config
├── README.md
└── .gitignore
```

---

## Python API Design

The Python API mirrors the Rust `quillmark` crate's public API with Pythonic conventions.

### Core Classes

#### 1. `Quillmark` - High-Level Engine

The main orchestration engine for managing backends and quills.

```python
from quillmark import Quillmark

# Create engine (auto-registers backends)
engine = Quillmark()

# Query registered backends and quills
backends = engine.registered_backends()  # -> list[str]
quills = engine.registered_quills()      # -> list[str]
```

**Constructor:**
```python
def __init__() -> None:
    """Create engine with auto-registered backends based on enabled features."""
```

**Methods:**
```python
def register_quill(quill: Quill) -> None:
    """Register a quill template with the engine."""

def workflow_from_quill_name(name: str) -> Workflow:
    """Load workflow by quill name (must be registered).
    
    Raises:
        QuillmarkError: If quill is not registered or backend unavailable
    """

def workflow_from_quill(quill: Quill) -> Workflow:
    """Load workflow from quill object (doesn't need to be registered)."""

def workflow_from_parsed(parsed: ParsedDocument) -> Workflow:
    """Load workflow from parsed document with QUILL field.
    
    Raises:
        QuillmarkError: If document lacks QUILL field
    """

def registered_backends() -> list[str]:
    """Get list of registered backend IDs."""

def registered_quills() -> list[str]:
    """Get list of registered quill names."""
```

---

#### 2. `Workflow` - Rendering Pipeline

Sealed workflow for executing the render pipeline.

```python
from quillmark import Workflow, OutputFormat

workflow = engine.workflow_from_quill_name("my-quill")

# Basic rendering
parsed = ParsedDocument.from_markdown(markdown)
result = workflow.render(parsed, OutputFormat.PDF)

# Dynamic assets
workflow.add_asset("chart.png", chart_bytes)
workflow.add_asset("data.csv", csv_bytes)

# Query properties
backend_id = workflow.backend_id()        # -> str
formats = workflow.supported_formats()    # -> list[OutputFormat]
quill_name = workflow.quill_name()        # -> str
```

**Methods:**
```python
def render(parsed: ParsedDocument, format: OutputFormat | None = None) -> RenderResult:
    """Render parsed document to artifacts.
    
    Args:
        parsed: Parsed markdown document
        format: Output format (defaults to first supported format)
    
    Returns:
        RenderResult with artifacts and warnings
    
    Raises:
        TemplateError: If template composition fails
        CompilationError: If backend compilation fails
    """

def render_source(content: str, format: OutputFormat | None = None) -> RenderResult:
    """Render pre-composed content (skip template processing)."""

def process_glue(markdown: str) -> str:
    """Process markdown through glue template, return composed output."""

def process_glue_parsed(parsed: ParsedDocument) -> str:
    """Process parsed document through glue template."""

def add_asset(filename: str, contents: bytes) -> None:
    """Add dynamic asset (mutates workflow)."""

def add_assets(assets: dict[str, bytes]) -> None:
    """Add multiple dynamic assets."""

def clear_assets() -> None:
    """Remove all dynamic assets."""

def add_font(filename: str, contents: bytes) -> None:
    """Add dynamic font."""

def add_fonts(fonts: dict[str, bytes]) -> None:
    """Add multiple dynamic fonts."""

def clear_fonts() -> None:
    """Remove all dynamic fonts."""

def backend_id() -> str:
    """Get backend identifier."""

def supported_formats() -> list[OutputFormat]:
    """Get supported output formats."""

def quill_name() -> str:
    """Get quill name."""

def dynamic_asset_names() -> list[str]:
    """Get list of dynamic asset filenames."""

def dynamic_font_names() -> list[str]:
    """Get list of dynamic font filenames."""
```

---

#### 3. `Quill` - Template Bundle

Represents a quill template bundle loaded from the filesystem.

```python
from quillmark import Quill

# Load from path
quill = Quill.from_path("path/to/quill")

# Access properties
name = quill.name                    # -> str
backend = quill.backend              # -> str
glue_template = quill.glue_template  # -> str
metadata = quill.metadata            # -> dict[str, Any]
```

**Static Methods:**
```python
@staticmethod
def from_path(path: str | Path) -> Quill:
    """Load quill from filesystem path.
    
    Raises:
        QuillmarkError: If path doesn't exist or quill is invalid
    """
```

**Properties:**
```python
@property
def name() -> str:
    """Quill name from Quill.toml"""

@property
def backend() -> str:
    """Backend identifier"""

@property
def glue_template() -> str:
    """Template content"""

@property
def metadata() -> dict[str, Any]:
    """Quill metadata from Quill.toml"""
```

---

#### 4. `ParsedDocument` - Parsed Markdown

Represents a parsed markdown document with frontmatter.

```python
from quillmark import ParsedDocument

# Parse markdown
parsed = ParsedDocument.from_markdown(markdown)

# Access fields
body = parsed.body()                        # -> str
title = parsed.get_field("title")           # -> Any | None
fields = parsed.fields()                    # -> dict[str, Any]
quill_tag = parsed.quill_tag()              # -> str | None
```

**Static Methods:**
```python
@staticmethod
def from_markdown(markdown: str) -> ParsedDocument:
    """Parse markdown with YAML frontmatter.
    
    Raises:
        ParseError: If YAML frontmatter is invalid
    """
```

**Methods:**
```python
def body() -> str:
    """Get document body content."""

def get_field(key: str) -> Any | None:
    """Get frontmatter field value."""

def fields() -> dict[str, Any]:
    """Get all frontmatter fields."""

def quill_tag() -> str | None:
    """Get QUILL field value if present."""
```

---

#### 5. `RenderResult` - Output Container

Container for rendered artifacts and diagnostics.

```python
result = workflow.render(parsed, OutputFormat.PDF)

# Access artifacts
for artifact in result.artifacts:
    print(f"Format: {artifact.output_format}")
    print(f"Size: {len(artifact.bytes)} bytes")
    artifact.save("output.pdf")

# Check warnings
for warning in result.warnings:
    print(f"{warning.severity}: {warning.message}")
```

**Properties:**
```python
@property
def artifacts() -> list[Artifact]:
    """Rendered output artifacts"""

@property
def warnings() -> list[Diagnostic]:
    """Non-fatal warnings"""
```

---

#### 6. `Artifact` - Single Output

Single rendered artifact with format metadata.

```python
artifact = result.artifacts[0]

# Access data
data = artifact.bytes           # -> bytes
fmt = artifact.output_format    # -> OutputFormat

# Save to file
artifact.save("output.pdf")
```

**Properties:**
```python
@property
def bytes() -> bytes:
    """Artifact binary data"""

@property
def output_format() -> OutputFormat:
    """Output format"""
```

**Methods:**
```python
def save(path: str | Path) -> None:
    """Save artifact to file."""
```

---

### Enums

#### `OutputFormat`

```python
from quillmark import OutputFormat

# Available formats
OutputFormat.PDF
OutputFormat.SVG
OutputFormat.TXT
```

#### `Severity`

```python
from quillmark import Severity

Severity.ERROR
Severity.WARNING
Severity.NOTE
```

---

### Supporting Types

#### `Diagnostic`

```python
@dataclass
class Diagnostic:
    severity: Severity
    message: str
    code: str | None = None
    primary: Location | None = None
    related: list[Location] = field(default_factory=list)
    hint: str | None = None
```

#### `Location`

```python
@dataclass
class Location:
    file: str
    line: int
    col: int
```

---

### Error Handling

Python exception hierarchy mapping Rust errors:

```python
from quillmark import (
    QuillmarkError,        # Base exception
    ParseError,            # YAML parsing failed
    TemplateError,         # Template rendering failed
    CompilationError,      # Backend compilation failed
)

try:
    result = workflow.render(parsed, OutputFormat.PDF)
except CompilationError as e:
    print(f"Compilation failed: {e}")
    for diag in e.diagnostics:
        print(f"  {diag.severity}: {diag.message}")
        if diag.primary:
            loc = diag.primary
            print(f"    at {loc.file}:{loc.line}:{loc.col}")
except QuillmarkError as e:
    print(f"Error: {e}")
```

**Exception Hierarchy:**
```
QuillmarkError (base)
├── ParseError
├── TemplateError
└── CompilationError
```

**Exception Attributes:**
- `QuillmarkError.message: str` - Error message
- `ParseError.diagnostic: Diagnostic` - Structured diagnostic
- `TemplateError.diagnostic: Diagnostic` - Structured diagnostic
- `CompilationError.diagnostics: list[Diagnostic]` - Multiple diagnostics

---

## PyO3 Bindings Implementation

### Module Structure

**`src/lib.rs`:**
```rust
use pyo3::prelude::*;

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
    m.add("QuillmarkError", m.py().get_type_bound::<QuillmarkError>())?;
    m.add("ParseError", m.py().get_type_bound::<ParseError>())?;
    m.add("TemplateError", m.py().get_type_bound::<TemplateError>())?;
    m.add("CompilationError", m.py().get_type_bound::<CompilationError>())?;
    
    Ok(())
}
```

### Wrapper Pattern

Each Python class wraps the corresponding Rust type:

**`src/engine.rs`:**
```rust
use pyo3::prelude::*;
use quillmark::Quillmark;

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
        let workflow = self.inner.workflow_from_quill_name(name)
            .map_err(convert_render_error)?;
        Ok(PyWorkflow { inner: workflow })
    }
    
    fn registered_backends(&self) -> Vec<String> {
        self.inner.registered_backends()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
    
    fn registered_quills(&self) -> Vec<String> {
        self.inner.registered_quills()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}
```

### Error Mapping

**`src/errors.rs`:**
```rust
use pyo3::prelude::*;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use quillmark_core::RenderError;

// Base exception
create_exception!(_quillmark, QuillmarkError, PyException);

// Specific exceptions
create_exception!(_quillmark, ParseError, QuillmarkError);
create_exception!(_quillmark, TemplateError, QuillmarkError);
create_exception!(_quillmark, CompilationError, QuillmarkError);

pub fn convert_render_error(err: RenderError) -> PyErr {
    match err {
        RenderError::InvalidFrontmatter { diag, .. } => {
            ParseError::new_err(diag.message.clone())
        }
        RenderError::TemplateFailed { diag, .. } => {
            TemplateError::new_err(diag.message.clone())
        }
        RenderError::CompilationFailed(count, diags) => {
            CompilationError::new_err(format!(
                "Compilation failed with {} error(s)",
                count
            ))
        }
        _ => QuillmarkError::new_err(err.to_string()),
    }
}
```

### Type Conversions

**Enum Conversions (`src/enums.rs`):**
```rust
use pyo3::prelude::*;
use quillmark_core::{OutputFormat, Severity};

#[pyclass(name = "OutputFormat")]
#[derive(Clone, Copy)]
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
```

**Bytes Handling:**
```rust
#[pymethods]
impl PyArtifact {
    #[getter]
    fn bytes(&self, py: Python) -> PyObject {
        // Zero-copy view of bytes
        PyBytes::new_bound(py, &self.inner.bytes).into()
    }
}
```

---

## Build Configuration

### `pyproject.toml`

```toml
[build-system]
requires = ["maturin>=1.7,<2.0"]
build-backend = "maturin"

[project]
name = "quillmark"
version = "0.1.0"
description = "Python bindings for Quillmark - template-first Markdown rendering"
authors = [{ name = "Quillmark Contributors" }]
readme = "README.md"
license = { text = "Apache-2.0" }
requires-python = ">=3.10"
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Python :: 3.13",
    "Programming Language :: Rust",
    "Topic :: Text Processing :: Markup",
]
keywords = ["markdown", "pdf", "typst", "rendering", "templates"]

[project.urls]
Homepage = "https://github.com/nibsbin/quillmark"
Repository = "https://github.com/nibsbin/quillmark"

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "pytest-cov>=5.0",
    "mypy>=1.8",
    "ruff>=0.3",
]

[tool.maturin]
features = ["pyo3/extension-module"]
python-source = "python"
module-name = "quillmark._quillmark"
include = ["python/quillmark/**/*.py", "python/quillmark/py.typed"]

[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py"]

[tool.mypy]
python_version = "3.10"
strict = true

[tool.ruff]
line-length = 100
target-version = "py39"

[tool.ruff.lint]
select = ["E", "F", "W", "I", "N", "UP"]
```

### `Cargo.toml`

```toml
[package]
name = "quillmark"
version = "0.1.0"
edition = "2021"
description = "Python bindings for Quillmark"
license = "Apache-2.0"

[lib]
name = "_quillmark"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.22", features = ["extension-module", "abi3-py39"] }
quillmark = { path = "../quillmark", features = ["typst"] }
quillmark-core = { path = "../quillmark-core" }

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = "z"
```

### Build Commands

```bash
# Development build (fast, installs in current venv)
maturin develop

# Release build
maturin build --release

# Build wheels for multiple Python versions
maturin build --release --interpreter python3.10 python3.10 python3.11 python3.12

# Build and publish to PyPI
maturin publish
```

---

## Development Workflow

### Setup with `uv`

```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create virtual environment
uv venv

# Activate
source .venv/bin/activate  # Linux/macOS
.venv\Scripts\activate     # Windows

# Install maturin
uv pip install maturin

# Build and install in development mode
maturin develop

# Install dev dependencies
uv pip install -e ".[dev]"
```

### Daily Development

```bash
# Run tests
uv run pytest

# Type checking
uv run mypy python/quillmark

# Linting and formatting
uv run ruff check python/
uv run ruff format python/

# Rebuild after Rust changes
maturin develop
```

---

## Testing Strategy

### Test Structure

```
tests/
├── conftest.py              # Shared fixtures
├── test_engine.py           # Engine tests
├── test_workflow.py         # Workflow tests
├── test_quill.py            # Quill loading tests
├── test_render.py           # End-to-end rendering
├── test_errors.py           # Error handling
└── fixtures/
    └── test-quill/          # Test quill bundles
```

### Test Examples

**`tests/test_engine.py`:**
```python
import pytest
from quillmark import Quillmark, Quill

def test_engine_creation():
    engine = Quillmark()
    assert "typst" in engine.registered_backends()
    assert len(engine.registered_quills()) == 0

def test_register_quill(tmp_path):
    engine = Quillmark()
    quill = create_test_quill(tmp_path)
    engine.register_quill(quill)
    assert quill.name in engine.registered_quills()
```

**`tests/test_workflow.py`:**
```python
from quillmark import Quillmark, ParsedDocument, OutputFormat

def test_end_to_end_render(test_quill_path):
    engine = Quillmark()
    quill = Quill.from_path(test_quill_path)
    engine.register_quill(quill)
    
    workflow = engine.workflow_from_quill_name(quill.name)
    parsed = ParsedDocument.from_markdown("# Hello\n\nWorld")
    result = workflow.render(parsed, OutputFormat.PDF)
    
    assert len(result.artifacts) == 1
    assert result.artifacts[0].output_format == OutputFormat.PDF
    assert len(result.artifacts[0].bytes) > 0
```

### Coverage Goals

- Overall: 85%+
- Core API: 90%+
- Error paths: 80%+

---

## Distribution & Packaging

### PyPI Distribution Strategy

**Wheel Types:**
1. **Binary wheels** for major platforms (built via CI):
   - `manylinux_2_17_x86_64` (Linux x86_64)
   - `manylinux_2_17_aarch64` (Linux ARM64)
   - `macosx_10_12_x86_64` (macOS Intel)
   - `macosx_11_0_arm64` (macOS Apple Silicon)
   - `win_amd64` (Windows x86_64)

2. **Source distribution (sdist)** as fallback

**Using `abi3`:**
- Build wheels with `abi3-py39` for forward compatibility
- Single wheel works across Python 3.10-3.13

### CI/CD Pipeline

**`.github/workflows/python.yml`:**
```yaml
name: Python Package CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  release:
    types: [published]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        python-version: ['3.10', '3.10', '3.11', '3.12']
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: astral-sh/setup-uv@v3
      - uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python-version }}
      
      - name: Install dependencies
        run: |
          uv venv
          uv pip install maturin pytest
          maturin develop
      
      - name: Run tests
        run: uv run pytest

  build-wheels:
    runs-on: ${{ matrix.os }}
    if: github.event_name == 'release'
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: astral-sh/setup-uv@v3
      
      - name: Build wheels
        run: |
          uv venv
          uv pip install maturin
          maturin build --release --interpreter python3.10 python3.10 python3.11 python3.12
      
      - uses: actions/upload-artifact@v4
        with:
          name: wheels-${{ matrix.os }}
          path: target/wheels/*.whl

  publish:
    needs: [test, build-wheels]
    runs-on: ubuntu-latest
    if: github.event_name == 'release'
    steps:
      - uses: actions/download-artifact@v4
        with:
          pattern: wheels-*
          merge-multiple: true
          path: dist/
      
      - uses: astral-sh/setup-uv@v3
      
      - name: Publish to PyPI
        env:
          MATURIN_PYPI_TOKEN: ${{ secrets.PYPI_TOKEN }}
        run: |
          uv pip install maturin
          maturin upload dist/*.whl
```

### Installation

```bash
# From PyPI (when published)
pip install quillmark
uv pip install quillmark

# With dev dependencies
pip install quillmark[dev]

# From source
git clone https://github.com/nibsbin/quillmark.git
cd quillmark/quillmark
maturin develop
```

---

## Documentation

### Documentation Strategy

Use standard Python docstrings with type hints. Documentation is primarily inline.

**Example:**
```python
def render(
    self,
    parsed: ParsedDocument,
    format: OutputFormat | None = None
) -> RenderResult:
    """Render parsed document to output artifacts.
    
    Args:
        parsed: Parsed markdown document with frontmatter
        format: Output format (defaults to first supported format)
    
    Returns:
        RenderResult containing artifacts and warnings
    
    Raises:
        TemplateError: If template composition fails
        CompilationError: If backend compilation fails
    
    Example:
        >>> engine = Quillmark()
        >>> quill = Quill.from_path("my-quill")
        >>> engine.register_quill(quill)
        >>> workflow = engine.workflow_from_quill_name("my-quill")
        >>> parsed = ParsedDocument.from_markdown("# Hello")
        >>> result = workflow.render(parsed, OutputFormat.PDF)
        >>> result.artifacts[0].save("output.pdf")
    """
```

### Type Stubs

**`python/quillmark/__init__.pyi`:**
```python
from typing import Any
from pathlib import Path

class Quillmark:
    def __init__(self) -> None: ...
    def register_quill(self, quill: Quill) -> None: ...
    def workflow_from_quill_name(self, name: str) -> Workflow: ...
    def workflow_from_quill(self, quill: Quill) -> Workflow: ...
    def registered_backends(self) -> list[str]: ...
    def registered_quills(self) -> list[str]: ...

class Workflow:
    def render(
        self,
        parsed: ParsedDocument,
        format: OutputFormat | None = None
    ) -> RenderResult: ...
    def add_asset(self, filename: str, contents: bytes) -> None: ...
    def backend_id(self) -> str: ...
    # ... other methods

# ... other types
```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
- [ ] Set up `quillmark/` directory structure
- [ ] Configure `pyproject.toml` and `Cargo.toml`
- [ ] Implement basic PyO3 module structure (`lib.rs`)
- [ ] Create error exception hierarchy
- [ ] Implement `PyQuillmark` wrapper
- [ ] Verify `maturin develop` works
- [ ] Basic smoke test

**Deliverable**: Minimal working package that can create an engine

### Phase 2: Core API (Week 3-4)
- [ ] Implement `PyWorkflow` wrapper
- [ ] Implement `PyQuill` wrapper
- [ ] Implement `PyParsedDocument` wrapper
- [ ] Implement output types (`PyRenderResult`, `PyArtifact`)
- [ ] Implement enum conversions (`PyOutputFormat`, `PySeverity`)
- [ ] Add comprehensive unit tests
- [ ] Type hints and stubs

**Deliverable**: Full API parity with Rust crate

### Phase 3: Dynamic Assets & Polish (Week 5)
- [ ] Implement dynamic asset support (`with_asset`, `with_font`)
- [ ] Optimize memory handling (zero-copy where possible)
- [ ] Integration tests
- [ ] Error handling tests

**Deliverable**: Feature-complete package

### Phase 4: Distribution (Week 6)
- [ ] Set up GitHub Actions CI/CD
- [ ] Configure multi-platform wheel building
- [ ] Test PyPI publishing (test.pypi.org)
- [ ] Documentation review
- [ ] Example scripts

**Deliverable**: Production-ready package

### Phase 5: Release (Week 7)
- [ ] Final testing on all platforms
- [ ] Version 0.1.0 release
- [ ] Publish to PyPI
- [ ] Update main repository README

**Deliverable**: Public release

---

## Example Usage

### Basic Rendering

```python
from quillmark import Quillmark, Quill, ParsedDocument, OutputFormat

# Create engine
engine = Quillmark()

# Load and register quill
quill = Quill.from_path("quills/letter")
engine.register_quill(quill)

# Parse markdown
markdown = """---
title: Hello World
author: Alice
---

# Introduction

This is a **test** document.
"""

parsed = ParsedDocument.from_markdown(markdown)

# Create workflow and render
workflow = engine.workflow_from_quill_name("letter")
result = workflow.render(parsed, OutputFormat.PDF)

# Save output
result.artifacts[0].save("output.pdf")
print(f"Generated {len(result.artifacts[0].bytes)} bytes")
```

### Dynamic Assets

```python
from quillmark import Quillmark, OutputFormat, ParsedDocument
from pathlib import Path

# Load chart image
chart_bytes = Path("chart.png").read_bytes()

# Create workflow with dynamic asset
engine = Quillmark()
workflow = engine.workflow_from_quill_name("report")

workflow.add_asset("chart.png", chart_bytes)

parsed = ParsedDocument.from_markdown("# Report\n\n![Chart](chart.png)")
result = workflow.render(parsed, OutputFormat.PDF)

result.artifacts[0].save("report.pdf")
```

### Batch Processing

```python
from quillmark import Quillmark, OutputFormat, ParsedDocument
from pathlib import Path
import concurrent.futures

engine = Quillmark()
workflow = engine.workflow_from_quill_name("letter")

def render_document(md_file: Path) -> Path:
    content = md_file.read_text()
    parsed = ParsedDocument.from_markdown(content)
    result = workflow.render(parsed, OutputFormat.PDF)
    
    output_path = md_file.with_suffix('.pdf')
    result.artifacts[0].save(output_path)
    return output_path

markdown_files = Path("documents").glob("*.md")

with concurrent.futures.ThreadPoolExecutor() as executor:
    outputs = list(executor.map(render_document, markdown_files))

print(f"Rendered {len(outputs)} documents")
```

---

## Migration from Rust

For Rust users, the Python API closely mirrors the Rust API:

| Rust | Python | Notes |
|------|--------|-------|
| `Quillmark::new()` | `Quillmark()` | Constructor syntax |
| `engine.register_quill(quill)` | `engine.register_quill(quill)` | Same |
| `engine.workflow_from_quill_name("name")?` | `engine.workflow_from_quill_name("name")` | Exceptions vs Results |
| `workflow.render(&parsed, Some(fmt))?` | `workflow.render(parsed, fmt)` | Optional = None |
| `result.artifacts[0].bytes` | `result.artifacts[0].bytes` | Returns Python bytes |
| `OutputFormat::Pdf` | `OutputFormat.PDF` | Naming convention |

---

## Glossary

- **Quill**: Template bundle containing glue template, assets, and configuration
- **Glue**: Backend-specific template file (e.g., `.typ` for Typst)
- **Workflow**: Configured rendering pipeline with backend and quill
- **Artifact**: Rendered output bytes with format metadata
- **Backend**: Rendering engine (e.g., Typst) that compiles glue to output
- **Dynamic Asset**: Runtime-injected file accessible during rendering

---

**Document Version**: 2.0  
**Last Updated**: 2025  
**Status**: Design Phase
