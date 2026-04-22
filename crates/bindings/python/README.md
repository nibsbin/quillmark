# Quillmark — Python bindings

Python bindings for Quillmark's format-first Markdown rendering engine.

Maintained by [TTQ](https://tonguetoquill.com).

## Installation

```bash
pip install quillmark
```

## Quick Start

```python
from quillmark import Quillmark, Document, OutputFormat

engine = Quillmark()
quill = engine.quill_from_path("path/to/quill")

markdown = """---
QUILL: my_quill
title: Hello World
---

# Hello
"""

parsed = Document.from_markdown(markdown)
result = quill.render(parsed, OutputFormat.PDF)
result.artifacts[0].save("output.pdf")

# Round-trip: mutate, emit, re-parse
parsed.set_field("title", "Updated")
emitted = parsed.to_markdown()
reparsed = Document.from_markdown(emitted)
assert reparsed.frontmatter["title"] == "Updated"
```

## API Overview

### `Quillmark`

```python
engine = Quillmark()
engine.registered_backends()      # ['typst']
quill = engine.quill_from_path("path/to/quill")
workflow = engine.workflow(quill)
```

### `Quill`

```python
quill = engine.quill_from_path("path")
result = quill.render(markdown_or_parsed, OutputFormat.PDF)
```

### `Workflow`

```python
workflow = engine.workflow(quill)
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
