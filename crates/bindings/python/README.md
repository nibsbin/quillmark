# Quillmark — Python bindings for Quillmark

Python bindings for the Quillmark template-first Markdown rendering engine.

## Installation

```bash
pip install quillmark
```

## Quick Start

```python
from quillmark import Quillmark, Quill, ParsedDocument, OutputFormat

# Create engine
engine = Quillmark()

# Load and register a quill
quill = Quill.from_path("path/to/quill")
engine.register_quill(quill)

# Parse markdown
markdown = """---
QUILL: my_quill
title: Hello World
---

# Hello

This is a test document.
"""
parsed = ParsedDocument.from_markdown(markdown)

# Create workflow and render
workflow = engine.workflow(parsed)  # Infers quill from QUILL tag
result = workflow.render(parsed, OutputFormat.PDF)

# Save output
result.artifacts[0].save("output.pdf")
```

## API Overview

The Python API provides opinionated visibility over the rendering workflow:

1. **Load Quill** - Load template bundles from the filesystem
2. **Parse Markdown** - Parse Markdown with YAML frontmatter into `ParsedDocument`
3. **Inspect Quill** - Retrieve quill properties (metadata, field schemas, supported formats)
4. **Create Workflow** - Build a rendering pipeline from quill or parsed document
5. **Render** - Generate output artifacts with configurable options

### Core Classes

#### `Quillmark` - Engine

Manages backends and quills.

```python
engine = Quillmark()
engine.register_quill(quill)
engine.registered_backends()  # ['typst']
engine.registered_quills()    # ['my_quill']

# Create workflows
workflow = engine.workflow(parsed)     # Infer from QUILL tag
workflow = engine.workflow("name") # By name
workflow = engine.workflow_from_quill(quill)       # By object
```

#### `Quill` - Template Bundle

Represents a quill loaded from the filesystem.

```python
quill = Quill.from_path("path/to/quill")

# Properties
quill.name              # Quill name
quill.backend           # Backend identifier (e.g., "typst")
quill.plate     # Template content
quill.example           # Example markdown content
quill.metadata          # Quill metadata dict
quill.schemas            # Field documentation dict
quill.supported_formats()  # [OutputFormat.PDF, OutputFormat.SVG]
```

#### `ParsedDocument` - Parsed Markdown

Represents parsed Markdown with frontmatter.

```python
parsed = ParsedDocument.from_markdown(markdown)

parsed.body()           # Document body (str | None)
parsed.quill_tag()      # QUILL field value (str | None)
parsed.get_field(key)   # Get specific field (Any | None)
parsed.fields           # All frontmatter fields (dict)
```

#### `Workflow` - Rendering Pipeline

Sealed workflow for rendering.

```python
workflow = engine.workflow("my_quill")

# Render
result = workflow.render(parsed, OutputFormat.PDF)

# Query properties
workflow.quill_name           # "my_quill"
workflow.backend_id           # "typst"
workflow.supported_formats    # [OutputFormat.PDF, OutputFormat.SVG]

# Process plate only (no compilation)
plate_output = workflow.process_plate(parsed)
```

**Note**: Dynamic asset and font injection is not currently supported in Python bindings. Assets must be included in the quill bundle.

#### `RenderResult` - Output Container

Contains rendered artifacts and diagnostics.

```python
result = workflow.render(parsed, OutputFormat.PDF)

for artifact in result.artifacts:
    print(f"Format: {artifact.output_format}")
    print(f"Size: {len(artifact.bytes)} bytes")
    artifact.save("output.pdf")

for warning in result.warnings:
    print(f"{warning.severity}: {warning.message}")
```

## Examples

See the [examples/](examples/) directory for complete examples:

- [`workflow_demo.py`](examples/workflow_demo.py) - Full workflow demonstration
- [`basic.py`](examples/basic.py) - Basic rendering example
- [`batch.py`](examples/batch.py) - Batch processing example

## Development

This repository uses `uv` for local development (https://astral.sh/uv).

Install uv (one-time):

```zsh
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Canonical development flow:

```zsh
# Create virtual environment
uv venv

# Install developer extras (includes maturin, pytest, mypy, ruff)
uv pip install -e ".[dev]"

# Run tests
uv run pytest
```

### Alternative: Without uv

```bash
# Create venv
python3 -m venv venv
source venv/bin/activate

# Install dependencies
pip install maturin pytest

# Build and install
maturin develop

# Run tests
pytest
```

## License

Apache-2.0