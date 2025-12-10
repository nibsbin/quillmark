# Python Package Design for Quillmark

> **Status**: Implemented
> **Package Name**: `quillmark`
> **Target**: Python 3.10+

> **For implementation details, see**: `crates/bindings/python/src/`

## Overview

This document outlines the design for `quillmark`, a Python package that exposes the Quillmark rendering engine to Python applications using PyO3 for Rust-Python bindings.

**Design Goals:**
- Mirror the public API of the `quillmark` Rust crate
- Provide Pythonic interfaces following Python conventions
- Minimize overhead through efficient PyO3 bindings
- Support cross-platform distribution via PyPI wheels

**Non-Goals:**
- Exposing low-level `quillmark-core` internals
- Supporting custom backend implementations in Python
- Async/streaming APIs (v1.0)

---

## Python API Design

The Python API mirrors the Rust `quillmark` crate with Pythonic conventions.

### Core Classes

#### 1. `Quillmark` - High-Level Engine

Main orchestration engine for managing backends and quills.

```python
from quillmark import Quillmark

engine = Quillmark()
backends = engine.registered_backends()  # list[str]
quills = engine.registered_quills()      # list[str]
```

**Methods:**
- `register_quill(quill)` - Register a quill template
- `workflow(name)` - Load workflow by quill name
- `workflow_from_quill(quill)` - Load workflow from quill object
- `workflow(parsed)` - Load workflow from parsed document with QUILL field
- `registered_backends()` - Get list of registered backend IDs
- `registered_quills()` - Get list of registered quill names

#### 2. `Workflow` - Rendering Pipeline

Sealed workflow for executing the render pipeline.

```python
workflow = engine.workflow("my-quill")
parsed = ParsedDocument.from_markdown(markdown)
result = workflow.render(parsed, OutputFormat.PDF)
```

**Methods:**
- `render(parsed, format)` - Render parsed document to artifacts
- `render_processed(content, format)` - Render pre-composed content
- `process_plate(parsed)` - Process through plate template
- `backend_id()` - Get backend identifier (property)
- `supported_formats()` - Get supported output formats (property)
- `quill_name()` - Get quill name (property)

**Note**: Dynamic asset and font injection methods are not currently supported in Python bindings.

#### 3. `Quill` - Template Bundle

Represents a quill template bundle from filesystem.

```python
quill = Quill.from_path("path/to/quill")
name = quill.name
backend = quill.backend
```

**Static Methods:**
- `from_path(path)` - Load quill from filesystem path

**Properties:**
- `name`, `backend`, `plate`, `metadata`, `schema`, `example`

#### 4. `ParsedDocument` - Parsed Markdown

```python
parsed = ParsedDocument.from_markdown(markdown)
body = parsed.body()
title = parsed.get_field("title")
```

**Static Methods:**
- `from_markdown(markdown)` - Parse markdown with YAML frontmatter

**Methods:**
- `body()` - Get document body content
- `get_field(key)` - Get frontmatter field value
- `fields()` - Get all frontmatter fields
- `quill_tag()` - Get QUILL field value if present

#### 5. `RenderResult`, `Artifact`

```python
result = workflow.render(parsed, OutputFormat.PDF)
for artifact in result.artifacts:
    artifact.save("output.pdf")
```

**Properties:**
- `RenderResult.artifacts`, `RenderResult.warnings`
- `Artifact.bytes`, `Artifact.output_format`

**Methods:**
- `Artifact.save(path)` - Save artifact to file

### Enums

- `OutputFormat.PDF`, `OutputFormat.SVG`, `OutputFormat.TXT`
- `Severity.ERROR`, `Severity.WARNING`, `Severity.NOTE`

### Error Handling

```python
from quillmark import QuillmarkError, ParseError, TemplateError, CompilationError

try:
    result = workflow.render(parsed, OutputFormat.PDF)
except CompilationError as e:
    for diag in e.diagnostics:
        print(f"{diag.severity}: {diag.message}")
```

**Exception Hierarchy:**
- `QuillmarkError` (base)
  - `ParseError`
  - `TemplateError`
  - `CompilationError`

---

## PyO3 Bindings Implementation

See `bindings/quillmark-python/src/` for complete implementation details.

**Module Structure:**
- `lib.rs` - PyO3 module entry point with class and exception registration
- `engine.rs`, `workflow.rs`, `quill.rs` - Core class wrappers
- `types.rs` - Output types (RenderResult, Artifact, Diagnostic)
- `enums.rs` - Enum conversions (OutputFormat, Severity)
- `errors.rs` - Exception definitions and error mapping

**Error Delegation:** Delegates error handling to core types. External errors (Rust `RenderError`) are converted to Python exceptions with `PyDiagnostic` wrapping `SerializableDiagnostic` from core. This ensures consistency across bindings and maintains a single source of truth for error structure.

---

## Build Configuration

**Tools:** PyO3, maturin, uv

**Key Files:**
- `pyproject.toml` - Python project configuration
- `Cargo.toml` - Rust dependencies

**Build Commands:**
```bash
maturin develop              # Development build
maturin build --release      # Release build
maturin publish              # Publish to PyPI
```

---

## Development Workflow

**Setup:**
```bash
uv venv
source .venv/bin/activate
uv pip install maturin
maturin develop
uv pip install -e ".[dev]"
```

**Daily Development:**
```bash
uv run pytest               # Run tests
uv run mypy python/quillmark  # Type checking
uv run ruff check python/   # Linting
```

---

## Distribution & Packaging

**PyPI Distribution:**
- Binary wheels for major platforms (Linux, macOS, Windows)
- Multiple Python versions (3.10+)
- Source distribution as fallback

**Installation:**
```bash
pip install quillmark
uv pip install quillmark
```
