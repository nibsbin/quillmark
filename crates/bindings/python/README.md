# Quillmark — Python bindings

Python bindings for Quillmark's format-first Markdown rendering engine.

Maintained by [TTQ](https://tonguetoquill.com).

## Installation

```bash
pip install quillmark
```

## Quick Start

```python
from quillmark import Quillmark, ParsedDocument, OutputFormat

engine = Quillmark()
quill = engine.quill_from_path("path/to/quill")

markdown = """---
QUILL: my_quill
title: Hello World
---

# Hello
"""

parsed = ParsedDocument.from_markdown(markdown)
result = quill.render(parsed, OutputFormat.PDF)
result.artifacts[0].save("output.pdf")
```

## API Overview

### `Quillmark`

```python
engine = Quillmark()
engine.registered_backends()      # ['typst']
quill = engine.quill_from_path("path/to/quill")
workflow = engine.workflow(quill) # for dynamic assets/fonts
```

### `Quill`

```python
Quill.from_path("path")           # convenience constructor
quill = engine.quill_from_path("path")
result = quill.render(markdown_or_parsed, OutputFormat.PDF)
```

### `Workflow`

Use workflow when adding dynamic assets/fonts:

```python
workflow = engine.workflow(quill)
workflow.add_asset("logo.png", logo_bytes)
workflow.add_font("Custom.ttf", font_bytes)
result = workflow.render(parsed, OutputFormat.PDF)
```

## Development

```bash
uv venv
uv pip install -e ".[dev]"
uv run pytest
```

## License

Apache-2.0
