# Python Library Design for Quillmark

## Executive Summary

This document outlines the design and implementation plan for `pyquillmark`, a Python library that provides Pythonic access to Quillmark's rendering functionality. The library will use PyO3 for Rust-Python bindings, maturin for building, and uv for development and dependency management.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Python API Design](#python-api-design)
4. [Rust-Python Bindings Strategy](#rust-python-bindings-strategy)
5. [Compilation and Build Process](#compilation-and-build-process)
6. [Development Workflow](#development-workflow)
7. [CI/CD Pipeline](#cicd-pipeline)
8. [Packaging and Distribution](#packaging-and-distribution)
9. [Testing Strategy](#testing-strategy)
10. [Documentation](#documentation)
11. [Implementation Roadmap](#implementation-roadmap)

---

## Overview

### Goals

- **Pythonic API**: Expose Quillmark's high-level API (`QuillmarkEngine` and `Workflow`) with Python idioms
- **Type Safety**: Leverage Python type hints and runtime type checking
- **Performance**: Minimal overhead through efficient PyO3 bindings
- **Developer Experience**: Modern tooling with `uv` for fast dependency management
- **Cross-Platform**: Support Linux, macOS, and Windows with pre-built wheels

### Non-Goals (DO NOTS)

- Expose low-level `quillmark-core` internals (keep focused on high-level API)
- Support custom backend implementation in Python (Rust only)
- Replicate all Rust-specific features (e.g., trait implementations)

---

## Architecture

### Component Structure

```
pyquillmark/
├── src/
│   └── lib.rs                 # PyO3 bindings root
├── python/
│   └── pyquillmark/
│       ├── __init__.py        # Public API exports
│       ├── types.py           # Type hints and enums
│       ├── errors.py          # Exception hierarchy
│       └── utils.py           # Helper utilities
├── tests/
│   ├── test_engine.py         # Engine tests
│   ├── test_workflow.py       # Workflow tests
│   └── fixtures/              # Test quills and markdown
├── examples/
│   ├── basic_usage.py
│   ├── dynamic_assets.py
│   └── batch_rendering.py
├── Cargo.toml                 # Rust dependencies
├── pyproject.toml             # Python project config (PEP 621)
├── README.md
└── docs/
    ├── installation.md
    ├── quickstart.md
    └── api.md
```

### Design Principles

1. **Mirror Rust API Structure**: Maintain conceptual mapping to Rust crate
2. **Pythonic Idioms**: Use Python conventions (snake_case, context managers, iterators)
3. **Error Handling**: Map Rust errors to Python exceptions with rich context
4. **Memory Management**: Leverage PyO3's automatic memory management
5. **Zero-Copy Where Possible**: Use bytes/memoryview for binary data

---

## Python API Design

### Core Classes

#### 1. `QuillmarkEngine` (Engine)

High-level engine for managing backends and quills.

```python
from pyquillmark import QuillmarkEngine, Quill, OutputFormat

# Create engine with auto-registered backends
engine = QuillmarkEngine()

# Register quills
quill = Quill.from_path("path/to/quill")
engine.register_quill(quill)

# Get registered backends and quills
backends = engine.registered_backends()  # -> List[str]
quills = engine.registered_quills()      # -> List[str]

# Load workflow by name or object
workflow = engine.load("my-quill")       # by name
workflow = engine.load(quill)            # by object
```

**Methods:**
- `__init__() -> None`: Create engine with auto-registered backends
- `register_quill(quill: Quill) -> None`: Register a quill template
- `load(quill_ref: Union[str, Quill]) -> Workflow`: Load workflow
- `registered_backends() -> List[str]`: List registered backend IDs
- `registered_quills() -> List[str]`: List registered quill names

---

#### 2. `Workflow` (Render Execution)

Sealed workflow for rendering markdown to various formats.

```python
from pyquillmark import Workflow, Quill, OutputFormat

# Create workflow directly
backend = "typst"  # or get from engine
quill = Quill.from_path("path/to/quill")
workflow = Workflow(backend, quill)

# Render markdown
result = workflow.render(
    markdown="# Hello\n\nWorld",
    format=OutputFormat.PDF
)

# Access artifacts
for artifact in result.artifacts:
    with open(f"output.{artifact.format.extension}", "wb") as f:
        f.write(artifact.bytes)

# Dynamic assets (builder pattern via method chaining)
workflow_with_assets = (
    workflow
    .with_asset("chart.png", chart_bytes)
    .with_asset("data.csv", csv_bytes)
)

result = workflow_with_assets.render(markdown, OutputFormat.PDF)

# Process glue (intermediate representation)
glue_output = workflow.process_glue(markdown)

# Query workflow properties
backend_id = workflow.backend_id()           # -> str
formats = workflow.supported_formats()       # -> List[OutputFormat]
quill_name = workflow.quill_name()           # -> str
```

**Methods:**
- `__init__(backend: str, quill: Quill) -> None`: Create workflow
- `render(markdown: str, format: Optional[OutputFormat] = None) -> RenderResult`: Render markdown
- `render_source(content: str, format: Optional[OutputFormat] = None) -> RenderResult`: Render pre-processed content
- `process_glue(markdown: str) -> str`: Process markdown through glue template
- `with_asset(filename: str, contents: bytes) -> Workflow`: Add dynamic asset (returns new instance)
- `with_assets(assets: Dict[str, bytes]) -> Workflow`: Add multiple assets
- `clear_assets() -> Workflow`: Remove all dynamic assets
- `backend_id() -> str`: Get backend identifier
- `supported_formats() -> List[OutputFormat]`: Get supported output formats
- `quill_name() -> str`: Get quill name

---

#### 3. `Quill` (Template Bundle)

Represents a quill template bundle.

```python
from pyquillmark import Quill

# Load from path
quill = Quill.from_path("path/to/quill")

# Access properties
name = quill.name                    # -> str
backend = quill.backend              # -> str
glue_template = quill.glue_template  # -> str
metadata = quill.metadata            # -> Dict[str, Any]

# Validation (automatic on load, can be called explicitly)
quill.validate()
```

**Properties:**
- `name: str`: Quill name from Quill.toml
- `backend: str`: Backend identifier
- `glue_template: str`: Template content
- `metadata: Dict[str, Any]`: Quill metadata from Quill.toml

**Methods:**
- `from_path(path: Union[str, Path]) -> Quill`: Load quill from filesystem
- `validate() -> None`: Validate quill structure (raises on error)

---

#### 4. `RenderResult` (Output Container)

Container for rendered artifacts and diagnostics.

```python
result = workflow.render(markdown, OutputFormat.PDF)

# Access artifacts
for artifact in result.artifacts:
    print(f"Format: {artifact.format}")
    print(f"Size: {len(artifact.bytes)} bytes")
    # artifact.bytes is a bytes object

# Access warnings
for warning in result.warnings:
    print(f"{warning.severity}: {warning.message}")
    if warning.location:
        print(f"  at {warning.location.file}:{warning.location.line}")
```

**Properties:**
- `artifacts: List[Artifact]`: Rendered output artifacts
- `warnings: List[Diagnostic]`: Non-fatal warnings

---

#### 5. `Artifact` (Output Artifact)

Single rendered artifact.

```python
artifact = result.artifacts[0]

# Access properties
data = artifact.bytes          # -> bytes
format = artifact.format       # -> OutputFormat

# Helper methods
artifact.save("output.pdf")    # Save to file
extension = artifact.extension # -> str (e.g., "pdf")
```

**Properties:**
- `bytes: bytes`: Artifact data
- `format: OutputFormat`: Output format

**Methods:**
- `save(path: Union[str, Path]) -> None`: Save artifact to file
- `extension() -> str`: Get file extension for format

---

### Enums and Types

#### `OutputFormat` (Enum)

```python
from pyquillmark import OutputFormat

# Available formats
OutputFormat.PDF
OutputFormat.SVG
OutputFormat.TXT

# Get extension
ext = OutputFormat.PDF.extension  # -> "pdf"

# String conversion
str(OutputFormat.PDF)  # -> "pdf"
```

#### `Severity` (Enum)

```python
from pyquillmark import Severity

Severity.ERROR
Severity.WARNING
Severity.NOTE
```

---

### Error Handling

Python-native exception hierarchy mapping Rust errors:

```python
from pyquillmark import (
    QuillmarkError,           # Base exception
    EngineCreationError,      # Engine initialization failed
    InvalidFrontmatterError,  # YAML parsing failed
    TemplateError,            # Template rendering failed
    CompilationError,         # Backend compilation failed
    FormatNotSupportedError,  # Unsupported output format
    UnsupportedBackendError,  # Backend not available
    DynamicAssetError,        # Dynamic asset collision
)

try:
    result = workflow.render(markdown, OutputFormat.PDF)
except CompilationError as e:
    print(f"Compilation failed: {e}")
    for diagnostic in e.diagnostics:
        print(f"  {diagnostic.severity}: {diagnostic.message}")
        if diagnostic.location:
            print(f"    at {diagnostic.location.file}:{diagnostic.location.line}")
except QuillmarkError as e:
    print(f"Error: {e}")
```

**Exception Hierarchy:**
```
QuillmarkError (base)
├── EngineCreationError
├── InvalidFrontmatterError
├── TemplateError
├── CompilationError
├── FormatNotSupportedError
├── UnsupportedBackendError
└── DynamicAssetError
```

All exceptions include:
- `message: str`: Error message
- `diagnostic: Optional[Diagnostic]`: Structured diagnostic (where applicable)
- `diagnostics: List[Diagnostic]`: Multiple diagnostics (CompilationError only)

---

### Additional Types

#### `Diagnostic` (Dataclass)

```python
@dataclass
class Diagnostic:
    severity: Severity
    message: str
    code: Optional[str] = None
    location: Optional[Location] = None
    related: List[Location] = field(default_factory=list)
    hint: Optional[str] = None
```

#### `Location` (Dataclass)

```python
@dataclass
class Location:
    file: str
    line: int
    column: int
```

---

## Rust-Python Bindings Strategy

### PyO3 Implementation

#### 1. Module Structure (`src/lib.rs`)

```rust
use pyo3::prelude::*;

#[pymodule]
fn pyquillmark(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register classes
    m.add_class::<PyQuillmarkEngine>()?;
    m.add_class::<PyWorkflow>()?;
    m.add_class::<PyQuill>()?;
    m.add_class::<PyRenderResult>()?;
    m.add_class::<PyArtifact>()?;
    
    // Register enums
    m.add_class::<PyOutputFormat>()?;
    m.add_class::<PySeverity>()?;
    
    // Register exceptions
    m.add("QuillmarkError", m.py().get_type_bound::<PyQuillmarkError>())?;
    m.add("CompilationError", m.py().get_type_bound::<PyCompilationError>())?;
    // ... other exceptions
    
    Ok(())
}
```

#### 2. Wrapper Classes

Each Python-exposed class wraps the corresponding Rust type:

```rust
#[pyclass(name = "QuillmarkEngine")]
struct PyQuillmarkEngine {
    inner: quillmark::QuillmarkEngine,
}

#[pymethods]
impl PyQuillmarkEngine {
    #[new]
    fn new() -> Self {
        Self {
            inner: quillmark::QuillmarkEngine::new(),
        }
    }
    
    fn register_quill(&mut self, quill: &PyQuill) {
        self.inner.register_quill(quill.inner.clone());
    }
    
    fn load(&self, quill_ref: QuillRefWrapper) -> PyResult<PyWorkflow> {
        let workflow = self.inner.load(quill_ref.to_rust())
            .map_err(|e| PyErr::from(e))?;
        Ok(PyWorkflow { inner: workflow })
    }
    
    fn registered_backends(&self) -> Vec<String> {
        self.inner.registered_backends()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
    
    fn registered_quills(&self) -> Vec<String> {
        self.inner.registered_quills()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}
```

#### 3. Type Conversions

**Union Types** (for flexible APIs):
```rust
use pyo3::types::{PyString, PyAny};

enum QuillRefWrapper {
    Name(String),
    Object(PyQuill),
}

impl<'py> FromPyObject<'py> for QuillRefWrapper {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if let Ok(s) = obj.extract::<String>() {
            Ok(QuillRefWrapper::Name(s))
        } else if let Ok(q) = obj.extract::<Py<PyQuill>>() {
            Ok(QuillRefWrapper::Object(q.borrow(obj.py()).clone()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected str or Quill"
            ))
        }
    }
}
```

**Bytes Handling**:
```rust
#[pymethods]
impl PyArtifact {
    #[getter]
    fn bytes(&self, py: Python) -> PyObject {
        // Zero-copy view when possible
        PyBytes::new_bound(py, &self.inner.bytes).into()
    }
}
```

#### 4. Error Mapping

```rust
use pyo3::exceptions;
use pyo3::create_exception;

// Base exception
create_exception!(pyquillmark, QuillmarkError, exceptions::PyException);

// Specific exceptions
create_exception!(pyquillmark, CompilationError, QuillmarkError);
create_exception!(pyquillmark, TemplateError, QuillmarkError);
// ... others

impl From<quillmark::RenderError> for PyErr {
    fn from(err: quillmark::RenderError) -> PyErr {
        match err {
            quillmark::RenderError::CompilationFailed(count, diags) => {
                let py_diags: Vec<PyDiagnostic> = diags.into_iter()
                    .map(|d| d.into())
                    .collect();
                
                PyCompilationError::new_err((
                    format!("Compilation failed with {} error(s)", count),
                    py_diags,
                ))
            }
            quillmark::RenderError::TemplateFailed { diag, .. } => {
                PyTemplateError::new_err((
                    diag.message.clone(),
                    PyDiagnostic::from(diag),
                ))
            }
            // ... other variants
            _ => QuillmarkError::new_err(err.to_string()),
        }
    }
}
```

---

## Compilation and Build Process

### Maturin Configuration

`pyproject.toml`:

```toml
[build-system]
requires = ["maturin>=1.7,<2.0"]
build-backend = "maturin"

[project]
name = "pyquillmark"
version = "0.1.0"
description = "Python bindings for Quillmark - a template-first Markdown rendering system"
authors = [
    { name = "Quillmark Contributors" }
]
readme = "README.md"
license = { text = "Apache-2.0" }
requires-python = ">=3.9"
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.9",
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
Documentation = "https://pyquillmark.readthedocs.io"

[project.optional-dependencies]
dev = [
    "pytest>=8.0",
    "pytest-cov>=4.1",
    "mypy>=1.8",
    "ruff>=0.3",
    "black>=24.0",
]
docs = [
    "sphinx>=7.0",
    "sphinx-rtd-theme>=2.0",
]

[tool.maturin]
features = ["pyo3/extension-module"]
python-source = "python"
module-name = "pyquillmark._pyquillmark"

# Build both pure-Python stub files and native extension
include = ["python/pyquillmark/**/*.py", "python/pyquillmark/py.typed"]

[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py"]

[tool.mypy]
python_version = "3.9"
strict = true
warn_return_any = true
warn_unused_configs = true

[tool.ruff]
line-length = 100
target-version = "py39"

[tool.ruff.lint]
select = ["E", "F", "W", "I", "N", "UP", "B", "A", "C4", "DTZ", "T10", "EM", "ISC", "ICN", "PIE", "PT", "Q", "RSE", "RET", "SIM", "ARG", "PTH", "ERA", "PD", "PGH", "PL", "TRY", "NPY", "RUF"]
ignore = ["ISC001"]  # Conflicts with formatter

[tool.black]
line-length = 100
target-version = ["py39"]
```

`Cargo.toml`:

```toml
[package]
name = "pyquillmark"
version = "0.1.0"
edition = "2021"

[lib]
name = "pyquillmark"
crate-type = ["cdylib"]

[dependencies]
pyo3 = { version = "0.22", features = ["extension-module", "abi3-py39"] }
quillmark = { path = "../quillmark", features = ["typst"] }
quillmark-core = { path = "../quillmark-core" }

[features]
default = []
extension-module = ["pyo3/extension-module"]

[profile.release]
lto = true
codegen-units = 1
strip = true
```

### Build Commands

```bash
# Development build (fast, debug symbols)
maturin develop

# Release build with optimizations
maturin build --release

# Build wheels for distribution
maturin build --release --strip

# Build for multiple Python versions
maturin build --release --interpreter python3.9 python3.10 python3.11 python3.12
```

---

## Development Workflow

### Using `uv` Package Manager

`uv` provides fast, reliable Python package management with Rust-powered performance.

#### Setup

```bash
# Install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create virtual environment
uv venv

# Activate
source .venv/bin/activate  # Linux/macOS
.venv\Scripts\activate     # Windows

# Install project in development mode
uv pip install -e ".[dev]"

# Install and build with maturin
uv pip install maturin
maturin develop
```

#### Daily Workflow

```bash
# Install dependencies
uv pip install -r requirements-dev.txt

# Update dependencies
uv pip compile pyproject.toml -o requirements.txt
uv pip compile pyproject.toml --extra dev -o requirements-dev.txt

# Run tests
uv run pytest

# Type checking
uv run mypy python/pyquillmark

# Linting and formatting
uv run ruff check python/
uv run ruff format python/

# Build documentation
uv run sphinx-build docs docs/_build
```

#### Lock File Management

```bash
# Generate lock file (deterministic builds)
uv pip compile pyproject.toml --generate-hashes -o requirements.lock

# Install from lock file
uv pip sync requirements.lock
```

---

## CI/CD Pipeline

### GitHub Actions Workflow

`.github/workflows/python.yml`:

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
  lint:
    name: Lint and Type Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install uv
        uses: astral-sh/setup-uv@v3
        
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'
          
      - name: Install dependencies
        run: |
          uv venv
          source .venv/bin/activate
          uv pip install -e ".[dev]"
          
      - name: Lint with ruff
        run: |
          source .venv/bin/activate
          ruff check python/
          
      - name: Format check with ruff
        run: |
          source .venv/bin/activate
          ruff format --check python/
          
      - name: Type check with mypy
        run: |
          source .venv/bin/activate
          mypy python/pyquillmark

  test:
    name: Test on ${{ matrix.os }} - Python ${{ matrix.python-version }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        python-version: ['3.9', '3.10', '3.11', '3.12']
        
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        
      - name: Install uv
        uses: astral-sh/setup-uv@v3
        
      - name: Set up Python ${{ matrix.python-version }}
        uses: actions/setup-python@v5
        with:
          python-version: ${{ matrix.python-version }}
          
      - name: Cache Rust dependencies
        uses: Swatinem/rust-cache@v2
        
      - name: Install dependencies and build
        run: |
          uv venv
          source .venv/bin/activate || .venv\Scripts\activate
          uv pip install maturin
          uv pip install -e ".[dev]"
          maturin develop --release
          
      - name: Run tests
        run: |
          source .venv/bin/activate || .venv\Scripts\activate
          pytest --cov=pyquillmark --cov-report=xml --cov-report=term
          
      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          file: ./coverage.xml
          flags: ${{ matrix.os }}-py${{ matrix.python-version }}

  build-wheels:
    name: Build wheels on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    if: github.event_name == 'release'
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        
      - name: Install uv
        uses: astral-sh/setup-uv@v3
        
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.11'
          
      - name: Build wheels
        run: |
          uv venv
          source .venv/bin/activate || .venv\Scripts\activate
          uv pip install maturin
          maturin build --release --strip --interpreter python3.9 python3.10 python3.11 python3.12
          
      - name: Upload wheels
        uses: actions/upload-artifact@v4
        with:
          name: wheels-${{ matrix.os }}
          path: target/wheels/*.whl

  publish:
    name: Publish to PyPI
    needs: [lint, test, build-wheels]
    runs-on: ubuntu-latest
    if: github.event_name == 'release'
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Download wheels
        uses: actions/download-artifact@v4
        with:
          pattern: wheels-*
          merge-multiple: true
          path: dist/
          
      - name: Install uv
        uses: astral-sh/setup-uv@v3
        
      - name: Publish to PyPI
        env:
          MATURIN_PYPI_TOKEN: ${{ secrets.PYPI_TOKEN }}
        run: |
          uv venv
          source .venv/bin/activate
          uv pip install maturin
          maturin upload dist/*.whl
```

### Release Workflow

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          draft: false
          prerelease: false
          generate_release_notes: true
```

---

## Packaging and Distribution

### PyPI Distribution

#### Package Metadata

The package will be distributed as:
- **Source distribution (sdist)**: For platforms without pre-built wheels
- **Binary wheels**: Pre-built for major platforms (Linux, macOS, Windows) × Python versions (3.9-3.12)

#### Wheel Naming Convention

```
pyquillmark-0.1.0-cp39-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl
pyquillmark-0.1.0-cp39-abi3-macosx_10_12_x86_64.whl
pyquillmark-0.1.0-cp39-abi3-win_amd64.whl
```

Using `abi3` allows a single wheel to work across Python versions.

#### Installation

```bash
# From PyPI (when published)
pip install pyquillmark
uv pip install pyquillmark

# With development dependencies
pip install pyquillmark[dev]
uv pip install "pyquillmark[dev]"

# From source
git clone https://github.com/nibsbin/quillmark.git
cd quillmark/pyquillmark
uv pip install maturin
maturin develop
```

### Version Management

- Follow Semantic Versioning (SemVer): `MAJOR.MINOR.PATCH`
- Keep Python package version in sync with Rust crate
- Use git tags for releases: `v0.1.0`

---

## Testing Strategy

### Test Structure

```
tests/
├── conftest.py                # Shared fixtures
├── test_engine.py             # Quillmark engine tests
├── test_workflow.py           # Workflow tests
├── test_quill.py              # Quill loading tests
├── test_render.py             # End-to-end rendering tests
├── test_errors.py             # Error handling tests
├── test_dynamic_assets.py     # Dynamic asset tests
├── test_types.py              # Type conversions tests
└── fixtures/
    ├── test-quill/
    │   ├── Quill.toml
    │   └── glue.typ
    ├── sample.md
    └── test_data/
```

### Test Categories

#### 1. Unit Tests

Test individual components in isolation:

```python
# test_engine.py
def test_engine_creation():
    engine = QuillmarkEngine()
    assert "typst" in engine.registered_backends()
    assert len(engine.registered_quills()) == 0

def test_register_quill(tmp_path):
    engine = QuillmarkEngine()
    quill = create_test_quill(tmp_path)
    engine.register_quill(quill)
    assert quill.name in engine.registered_quills()
```

#### 2. Integration Tests

Test component interactions:

```python
# test_workflow.py
def test_end_to_end_render(tmp_path):
    engine = QuillmarkEngine()
    quill = create_test_quill(tmp_path)
    engine.register_quill(quill)
    
    workflow = engine.load(quill.name)
    result = workflow.render("# Hello\n\nWorld", OutputFormat.PDF)
    
    assert len(result.artifacts) == 1
    assert result.artifacts[0].format == OutputFormat.PDF
    assert len(result.artifacts[0].bytes) > 0
```

#### 3. Error Handling Tests

```python
# test_errors.py
def test_compilation_error():
    workflow = create_workflow()
    with pytest.raises(CompilationError) as exc_info:
        workflow.render("{{ invalid template }}", OutputFormat.PDF)
    
    assert len(exc_info.value.diagnostics) > 0
    assert exc_info.value.diagnostics[0].severity == Severity.ERROR
```

#### 4. Type Tests (with mypy)

```python
# test_types.py
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # Type-only tests for mypy
    reveal_type(engine.load("quill"))  # Should be Workflow
    reveal_type(workflow.render("md"))  # Should be RenderResult
```

### Coverage Goals

- Overall: 90%+
- Core API: 95%+
- Error paths: 85%+

### Test Execution

```bash
# Run all tests
uv run pytest

# Run with coverage
uv run pytest --cov=pyquillmark --cov-report=html

# Run specific test file
uv run pytest tests/test_workflow.py

# Run with verbose output
uv run pytest -v

# Run type checks
uv run mypy python/pyquillmark
```

---

## Documentation

### Documentation Structure

```
docs/
├── index.rst                  # Main documentation page
├── installation.md            # Installation instructions
├── quickstart.md              # Getting started guide
├── user_guide/
│   ├── engine.md              # Engine usage
│   ├── workflows.md           # Workflow usage
│   ├── quills.md              # Working with quills
│   ├── dynamic_assets.md      # Dynamic assets
│   └── error_handling.md      # Error handling
├── api/
│   ├── index.rst              # API reference index
│   ├── engine.rst             # Quillmark class
│   ├── workflow.rst           # Workflow class
│   ├── types.rst              # Types and enums
│   └── errors.rst             # Exceptions
├── examples/
│   ├── basic.md               # Basic usage examples
│   ├── advanced.md            # Advanced patterns
│   └── batch.md               # Batch processing
└── development/
    ├── contributing.md        # Contribution guide
    ├── building.md            # Building from source
    └── testing.md             # Testing guide
```

### Documentation Tools

- **Sphinx**: Documentation generator
- **sphinx-rtd-theme**: ReadTheDocs theme
- **autodoc**: Auto-generate from docstrings
- **napoleon**: Google/NumPy style docstrings

### Docstring Style

Use Google-style docstrings:

```python
def render(
    self,
    markdown: str,
    format: Optional[OutputFormat] = None
) -> RenderResult:
    """Render markdown to specified output format.
    
    Args:
        markdown: Markdown content with optional YAML frontmatter
        format: Output format (defaults to first supported format)
        
    Returns:
        RenderResult containing artifacts and warnings
        
    Raises:
        TemplateError: If template rendering fails
        CompilationError: If backend compilation fails
        
    Example:
        >>> workflow = engine.load("my-quill")
        >>> result = workflow.render("# Hello", OutputFormat.PDF)
        >>> result.artifacts[0].save("output.pdf")
    """
```

### Building Documentation

```bash
# Install documentation dependencies
uv pip install -e ".[docs]"

# Build HTML documentation
uv run sphinx-build docs docs/_build/html

# Serve locally
python -m http.server -d docs/_build/html 8000
```

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

- [ ] **1.1** Set up project structure (`pyquillmark/` directory)
- [ ] **1.2** Configure `pyproject.toml` and `Cargo.toml`
- [ ] **1.3** Implement basic PyO3 bindings for core types
  - [ ] `PyQuillmark` wrapper
  - [ ] `PyWorkflow` wrapper
  - [ ] `PyQuill` wrapper
- [ ] **1.4** Implement error mapping (Rust → Python exceptions)
- [ ] **1.5** Basic type conversions (enums, structs)
- [ ] **1.6** Set up `uv` development environment
- [ ] **1.7** Create minimal test suite
- [ ] **1.8** Verify `maturin develop` works

**Deliverable**: Basic working Python bindings that can create an engine and load a quill

### Phase 2: Core API (Week 3-4)

- [ ] **2.1** Complete `Quillmark` class implementation
  - [ ] `register_quill()`
  - [ ] `load()` with QuillRef support
  - [ ] `registered_backends()` / `registered_quills()`
- [ ] **2.2** Complete `Workflow` class implementation
  - [ ] `render()`
  - [ ] `render_source()`
  - [ ] `process_glue()`
  - [ ] Property getters
- [ ] **2.3** Implement `Quill` class
  - [ ] `from_path()`
  - [ ] Property access
  - [ ] Validation
- [ ] **2.4** Implement output types
  - [ ] `RenderResult`
  - [ ] `Artifact` with `save()` method
  - [ ] `Diagnostic` and `Location`
- [ ] **2.5** Complete error hierarchy
- [ ] **2.6** Add comprehensive unit tests
- [ ] **2.7** Type hints and stubs

**Deliverable**: Full API parity with Rust crate's high-level API

### Phase 3: Dynamic Assets & Advanced Features (Week 5)

- [ ] **3.1** Implement dynamic asset support
  - [ ] `with_asset()` builder method
  - [ ] `with_assets()` batch method
  - [ ] `clear_assets()`
  - [ ] Collision detection
- [ ] **3.2** Memory optimization
  - [ ] Zero-copy bytes handling
  - [ ] Efficient string conversions
- [ ] **3.3** Add integration tests
- [ ] **3.4** Performance benchmarks

**Deliverable**: Feature-complete Python library

### Phase 4: Build & CI/CD (Week 6)

- [ ] **4.1** Set up GitHub Actions workflows
  - [ ] Linting and type checking
  - [ ] Multi-platform testing (Linux, macOS, Windows)
  - [ ] Multi-version testing (Python 3.9-3.12)
- [ ] **4.2** Configure wheel building
  - [ ] manylinux wheels
  - [ ] macOS universal2 wheels
  - [ ] Windows wheels
- [ ] **4.3** Set up PyPI publishing
  - [ ] Test PyPI deployment
  - [ ] Production PyPI deployment
- [ ] **4.4** Coverage reporting (Codecov)
- [ ] **4.5** Release automation

**Deliverable**: Automated CI/CD pipeline

### Phase 5: Documentation & Examples (Week 7)

- [ ] **5.1** Write comprehensive docstrings
- [ ] **5.2** Set up Sphinx documentation
- [ ] **5.3** Write user guide
  - [ ] Installation
  - [ ] Quickstart
  - [ ] Detailed usage guides
- [ ] **5.4** API reference documentation
- [ ] **5.5** Create examples
  - [ ] Basic usage
  - [ ] Dynamic assets
  - [ ] Batch processing
  - [ ] Error handling
- [ ] **5.6** README.md for PyPI
- [ ] **5.7** Deploy documentation (ReadTheDocs)

**Deliverable**: Complete documentation

### Phase 6: Polish & Release (Week 8)

- [ ] **6.1** Performance optimization
- [ ] **6.2** Final testing on all platforms
- [ ] **6.3** Security audit
- [ ] **6.4** License compliance check
- [ ] **6.5** Version 0.1.0 release
  - [ ] Git tag
  - [ ] GitHub release
  - [ ] PyPI publish
- [ ] **6.6** Announcement and promotion

**Deliverable**: Public v0.1.0 release on PyPI

---

## Development Guidelines

### Code Style

- **Python**: Follow PEP 8, use `ruff` for linting and formatting
- **Rust**: Follow Rust conventions, use `rustfmt` and `clippy`
- **Type Hints**: All public APIs must have type hints
- **Docstrings**: All public APIs must have docstrings

### Performance Considerations

1. **Minimize Python-Rust crossings**: Batch operations when possible
2. **Zero-copy when possible**: Use `PyBytes` and memoryview
3. **GIL management**: Release GIL for CPU-intensive operations
4. **Lazy evaluation**: Defer expensive operations until needed

### Security

1. **Input validation**: Validate all inputs from Python side
2. **Path traversal**: Sanitize file paths
3. **Memory safety**: Leverage Rust's safety guarantees
4. **Dependency auditing**: Regular `cargo audit` and `pip-audit`

### Compatibility

- **Python versions**: Support 3.9+ (current stable releases)
- **Platforms**: Linux (x86_64, aarch64), macOS (x86_64, arm64), Windows (x86_64)
- **Rust version**: MSRV 1.70+ (aligned with PyO3)

---

## Future Enhancements

### Post-v0.1.0 Roadmap

1. **Performance Optimizations**
   - Parallel rendering for batch operations
   - Caching compiled templates
   - Memory pooling for artifacts

2. **Enhanced Type Safety**
   - Runtime type validation with Pydantic
   - Stricter type stubs

3. **Additional APIs**
   - Async/await support (`async def render()`)
   - Streaming output for large documents
   - Progress callbacks

4. **Developer Experience**
   - CLI tool for common operations
   - Jupyter notebook integration
   - VS Code extension

5. **Extended Backend Support**
   - When new backends are added to Rust crate, expose in Python

---

## Appendix

### A. Example Workflows

#### Basic Usage Example

`examples/basic_usage.py`:

```python
from pyquillmark import QuillmarkEngine, OutputFormat

# Create engine
engine = QuillmarkEngine()

# Load quill
from pyquillmark import Quill
quill = Quill.from_path("quills/letter")
engine.register_quill(quill)

# Render
workflow = engine.load("letter")
markdown = """---
title: Hello World
author: Alice
---

# Introduction

This is a **test** document.
"""

result = workflow.render(markdown, OutputFormat.PDF)

# Save output
result.artifacts[0].save("output.pdf")
print(f"Generated {len(result.artifacts[0].bytes)} bytes")
```

#### Dynamic Assets Example

`examples/dynamic_assets.py`:

```python
from pyquillmark import QuillmarkEngine, OutputFormat
import matplotlib.pyplot as plt
from io import BytesIO

# Generate chart
fig, ax = plt.subplots()
ax.plot([1, 2, 3, 4], [1, 4, 2, 3])

chart_buffer = BytesIO()
fig.savefig(chart_buffer, format='png')
chart_bytes = chart_buffer.getvalue()

# Render with dynamic asset
engine = QuillmarkEngine()
workflow = engine.load("report")

result = (
    workflow
    .with_asset("chart.png", chart_bytes)
    .render(markdown, OutputFormat.PDF)
)

result.artifacts[0].save("report.pdf")
```

#### Batch Processing Example

`examples/batch_rendering.py`:

```python
from pyquillmark import QuillmarkEngine, OutputFormat
from pathlib import Path
import concurrent.futures

engine = QuillmarkEngine()
workflow = engine.load("letter")

markdown_files = Path("documents").glob("*.md")

def render_document(md_file):
    content = md_file.read_text()
    result = workflow.render(content, OutputFormat.PDF)
    output_path = md_file.with_suffix('.pdf')
    result.artifacts[0].save(output_path)
    return output_path

# Parallel rendering
with concurrent.futures.ThreadPoolExecutor() as executor:
    outputs = list(executor.map(render_document, markdown_files))

print(f"Rendered {len(outputs)} documents")
```

### B. Migration from Rust

For existing Rust users, the Python API closely mirrors the Rust API:

| Rust | Python | Notes |
|------|--------|-------|
| `Quillmark::new()` | `Quillmark()` | Constructor syntax |
| `engine.register_quill(quill)` | `engine.register_quill(quill)` | Same |
| `engine.load("name")?` | `engine.load("name")` | Exceptions vs Results |
| `workflow.render(md, Some(fmt))?` | `workflow.render(md, fmt)` | Optional args are None |
| `result.artifacts` | `result.artifacts` | Same field access |
| `artifact.bytes` | `artifact.bytes` | Returns Python bytes |
| `OutputFormat::Pdf` | `OutputFormat.PDF` | Enum naming convention |

### C. Glossary

- **Quill**: A template bundle containing glue template, assets, and configuration
- **Glue**: Backend-specific template file (e.g., `.typ` for Typst)
- **Workflow**: Configured rendering pipeline with backend and quill
- **Artifact**: Rendered output bytes with format metadata
- **Backend**: Rendering engine (e.g., Typst) that compiles glue to output
- **Dynamic Asset**: Runtime-injected file accessible during rendering

---

**Document Version**: 1.0  
**Last Updated**: 2024  
**Status**: Planning/Design Phase
