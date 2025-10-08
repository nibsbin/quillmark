# PyQuillmark - Python Bindings for Quillmark

Python bindings for [Quillmark](https://github.com/nibsbin/quillmark), a template-first Markdown rendering system.

## Installation

```bash
pip install pyquillmark
```

## Quick Start

```python
from pyquillmark import Quillmark, Quill, ParsedDocument, OutputFormat

# Create engine
engine = Quillmark()

# Load and register quill
quill = Quill.from_path("path/to/quill")
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
workflow = engine.workflow_from_quill_name("my-quill")
result = workflow.render(parsed, OutputFormat.PDF)

# Save output
result.artifacts[0].save("output.pdf")
```

## Features

- Template-first Markdown rendering
- PDF and SVG output via Typst backend
- Dynamic asset and font support
- Python 3.10+ support
- Type hints included

## License

Apache-2.0
