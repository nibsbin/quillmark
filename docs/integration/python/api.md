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

    def workflow(self, quill: Quill) -> Workflow: ...

    def registered_backends(self) -> list[str]: ...
```

### `Quill`

Obtained via `engine.quill_from_path(path)`.

```python
class Quill:
    name: str
    backend: str
    plate: str | None
    metadata: dict
    schema: str           # public schema as YAML text
    defaults: dict        # field default values
    examples: dict        # field example value lists
    example: str | None   # raw example document string
    print_tree: str       # quill file tree

    def supported_formats(self) -> list[OutputFormat]: ...

    def render(
        self,
        parsed: ParsedDocument,
        format: OutputFormat | None = None,
    ) -> RenderResult: ...

    def open(self, parsed: ParsedDocument) -> RenderSession:
        """Open a render session for page inspection before rendering."""
```

### `Workflow`

```python
class Workflow:
    backend_id: str       # property
    supported_formats: list[OutputFormat]  # property
    quill_ref: str        # property

    def render(self, parsed: ParsedDocument, format: OutputFormat | None = None) -> RenderResult: ...
    def open(self, parsed: ParsedDocument) -> RenderSession: ...
    def dry_run(self, parsed: ParsedDocument) -> None:
        """Validate without compiling. Raises QuillmarkError on failure."""
```

### `RenderSession`

Obtained via `quill.open(parsed)` or `workflow.open(parsed)`. Useful for page-range rendering.

```python
session = quill.open(parsed)
print(session.page_count)
result = session.render(OutputFormat.PDF, pages=[0, 1])
```

```python
class RenderSession:
    page_count: int  # property

    def render(
        self,
        format: OutputFormat | None = None,
        pages: list[int] | None = None,
    ) -> RenderResult: ...
```

### `ParsedDocument`

```python
parsed = ParsedDocument.from_markdown(markdown)  # raises ParseError on failure
parsed.quill_ref()     # → str
parsed.fields          # property → dict
parsed.body()          # → str | None
parsed.get_field(key)  # → value | None
```

### `RenderResult` and `Artifact`

```python
result.artifacts       # list[Artifact]
result.warnings        # list[Diagnostic]
result.output_format   # OutputFormat

artifact.bytes         # bytes
artifact.output_format # OutputFormat
artifact.mime_type     # str (e.g. "application/pdf")
artifact.save(path)    # write bytes to file
```

### `Diagnostic` and `Location`

```python
diag.severity      # Severity
diag.message       # str
diag.code          # str | None
diag.primary       # Location | None
diag.hint          # str | None
diag.source_chain  # list[str]

loc.file  # str
loc.line  # int
loc.col   # int
```

## Enums

```python
OutputFormat.PDF   # application/pdf
OutputFormat.SVG   # image/svg+xml
OutputFormat.PNG   # image/png
OutputFormat.TXT   # text/plain
OutputFormat.all() # → list[OutputFormat]

Severity.ERROR
Severity.WARNING
Severity.NOTE
Severity.all()     # → list[Severity]
```

## Errors

| Exception | Raised when |
|---|---|
| `QuillmarkError` | Base class; validation, engine, or workflow failures |
| `ParseError` | Markdown/frontmatter parse failure; has `.diagnostic` |
| `TemplateError` | Template processing failure |
| `CompilationError` | Backend compilation failure; has `.diagnostics` (list) |
