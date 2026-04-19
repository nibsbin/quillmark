# Python API Reference

Complete reference for the Quillmark Python API.

## Installation

```bash
uv pip install quillmark
```

## Quick Example

```python
from quillmark import Quillmark, ParsedDocument, OutputFormat

engine = Quillmark()
quill = engine.quill_from_path("path/to/quill")

markdown = """---
QUILL: my_quill
title: My Document
---
# Content
"""

parsed = ParsedDocument.from_markdown(markdown)
result = quill.render(parsed, OutputFormat.PDF)
result.artifacts[0].save("output.pdf")
```

## Core Classes

### `Quillmark`

```python
class Quillmark:
    def __init__(self) -> None: ...

    def quill_from_path(self, path: str | Path) -> Quill:
        """Load a quill and attach backend (render-ready)."""

    def workflow(self, quill: Quill) -> Workflow:
        """Create workflow for dynamic asset/font injection."""

    def registered_backends(self) -> list[str]: ...
```

### `Quill`

```python
class Quill:
    @staticmethod
    def from_path(path: str | Path) -> Quill:
        """Load quill metadata/files only (no backend attached)."""

    def render(
        self,
        input: str | ParsedDocument,
        format: OutputFormat | None = None,
    ) -> RenderResult:
        """Render directly with this quill.

        Raises QuillmarkError if the quill was created with Quill.from_path()
        instead of engine.quill_from_path().
        """
```

### `Workflow`

Use when you need runtime assets/fonts:

```python
workflow = engine.workflow(quill)
workflow.add_asset("logo.png", logo_bytes)
workflow.add_font("Custom.ttf", font_bytes)
workflow.dry_run(parsed)
result = workflow.render(parsed, OutputFormat.PDF)
```

### `ParsedDocument`

```python
parsed = ParsedDocument.from_markdown(markdown)
parsed.quill_ref()
parsed.fields
parsed.body()
```

## Diagnostics and Errors

- `ParseError`: markdown/frontmatter parsing failures
- `QuillmarkError`: validation/rendering/workflow failures
- `RenderResult.warnings`: non-fatal diagnostics (including QUILL ref mismatch warnings)

## Schema Access

Python exposes schema as YAML text:

```python
schema_yaml = quill.schema
```

