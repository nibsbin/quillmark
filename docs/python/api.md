# Python API Reference

Complete reference for the Quillmark Python API.

## Installation

```bash
# Using uv (recommended)
uv pip install quillmark

# Using pip
pip install quillmark
```

## Quick Example

```python
from quillmark import Quillmark, ParsedDocument, OutputFormat, Quill

# Create engine and load quill
engine = Quillmark()
quill = Quill.from_path("path/to/quill")
engine.register_quill(quill)

# Parse markdown
markdown = """---
title: My Document
---
# Content
"""
parsed = ParsedDocument.from_markdown(markdown)

# Render
workflow = engine.workflow("my-quill")
workflow.dry_run(parsed)  # fast validation
result = workflow.render(parsed, OutputFormat.PDF)

# Save output
result.artifacts[0].save("output.pdf")
```

## Core Classes

### Quillmark

High-level engine for managing backends and quills.

```python
class Quillmark:
    def __init__(self) -> None:
        """Create engine with auto-registered backends."""

    def register_quill(self, quill: Quill) -> None:
        """Register a quill template.

        Raises:
            QuillmarkError: If validation fails
        """

    def workflow(self, quill_ref: str | Quill | ParsedDocument) -> Workflow:
        """Get workflow for rendering.

        Args:
            quill_ref: Quill name (str) or reference (e.g. "name@1.0"), Quill object, or ParsedDocument

        Returns:
            Workflow for rendering

        Raises:
            QuillmarkError: If quill not found
            TypeError: If invalid argument type
        """

    def registered_backends(self) -> list[str]:
        """Get registered backend IDs."""

    def registered_quills(self) -> list[str]:
        """Get registered quill names."""

    def get_quill(self, name_or_ref: str) -> Quill | None:
        """Get a registered quill by name or reference.

        Args:
            name_or_ref: Quill name (str) or reference (e.g. "name@1.0")

        Returns:
            Quill object or None if not found
        """
```

**Example:**

```python
engine = Quillmark()

# Check available backends
print(engine.registered_backends())  # ['typst', 'acroform']

# Load and register quill
quill = Quill.from_path("my-quill/")
engine.register_quill(quill)

# Check registered quills
print(engine.registered_quills())  # ['my-quill']

# Create workflow (multiple ways)
# Create workflow (multiple ways)
workflow = engine.workflow("my-quill")       # by name (latest)
workflow = engine.workflow("my-quill@1.0")   # by reference (specific version)
workflow = engine.workflow(quill)            # by Quill object
workflow = engine.workflow(parsed_document)  # by ParsedDocument (uses QUILL field)

# Retrieve quill
quill = engine.get_quill("my-quill")
quill_v1 = engine.get_quill("my-quill@1.0")
```

### Workflow

Sealed rendering pipeline for a specific quill.

```python
class Workflow:
    def render(
        self,
        parsed: ParsedDocument,
        format: OutputFormat | None = None
    ) -> RenderResult:
        """Render document to artifacts.

        Args:
            parsed: Parsed markdown document
            format: Output format (default: first supported)

        Returns:
            RenderResult with artifacts

        Raises:
            QuillmarkError: If rendering fails
        """

    def dry_run(self, parsed: ParsedDocument) -> None:
        """Validate document without compilation.

        Args:
            parsed: Parsed markdown document

        Raises:
            QuillmarkError: If validation fails
        """

    @property
    def backend_id(self) -> str:
        """Backend identifier."""

    @property
    def supported_formats(self) -> list[OutputFormat]:
        """Supported output formats."""

    @property
    def quill_name(self) -> str:
        """Quill name."""

    # Dynamic Asset Methods
    def add_asset(self, filename: str, contents: bytes) -> None:
        """Add a dynamic asset to the workflow.

        Args:
            filename: Name of the asset file (e.g., "logo.png")
            contents: Binary contents of the asset

        Raises:
            QuillmarkError: If an asset with the same filename already exists
        """

    def add_assets(self, assets: list[tuple[str, bytes]]) -> None:
        """Add multiple dynamic assets at once.

        Args:
            assets: List of tuples (filename, contents)

        Raises:
            QuillmarkError: If any asset filename collides
        """

    def clear_assets(self) -> None:
        """Clear all dynamic assets from the workflow."""

    def dynamic_asset_names(self) -> list[str]:
        """Get list of dynamic asset filenames currently in the workflow."""

    # Dynamic Font Methods
    def add_font(self, filename: str, contents: bytes) -> None:
        """Add a dynamic font to the workflow.

        Args:
            filename: Name of the font file (e.g., "custom.ttf")
            contents: Binary contents of the font

        Raises:
            QuillmarkError: If a font with the same filename already exists
        """

    def add_fonts(self, fonts: list[tuple[str, bytes]]) -> None:
        """Add multiple dynamic fonts at once.

        Args:
            fonts: List of tuples (filename, contents)

        Raises:
            QuillmarkError: If any font filename collides
        """

    def clear_fonts(self) -> None:
        """Clear all dynamic fonts from the workflow."""

    def dynamic_font_names(self) -> list[str]:
        """Get list of dynamic font filenames currently in the workflow."""
```

**Example:**

```python
workflow = engine.workflow("my-quill")

print(f"Backend: {workflow.backend_id}")  # 'typst'
print(f"Formats: {workflow.supported_formats}")  # [PDF, SVG]

# Validate before rendering
workflow.dry_run(parsed)

# Render to specific format
result = workflow.render(parsed, OutputFormat.PDF)
```

**Dynamic Assets Example:**

```python
workflow = engine.workflow("my-quill")

# Add a single asset
with open("logo.png", "rb") as f:
    workflow.add_asset("logo.png", f.read())

# Add multiple assets at once
assets = [
    ("image1.png", image1_bytes),
    ("image2.png", image2_bytes),
]
workflow.add_assets(assets)

# Check what assets are loaded
print(workflow.dynamic_asset_names())  # ['logo.png', 'image1.png', 'image2.png']

# Clear all assets
workflow.clear_assets()
```

### Quill

Template bundle containing plate templates and assets.

```python
class Quill:
    @staticmethod
    def from_path(path: str | Path) -> Quill:
        """Load quill from filesystem.

        Args:
            path: Path to quill directory

        Returns:
            Loaded quill

        Raises:
            QuillmarkError: If invalid or missing
        """

    @property
    def name(self) -> str:
        """Quill name from Quill.yaml."""

    @property
    def backend(self) -> str | None:
        """Backend identifier."""

    @property
    def plate(self) -> str | None:
        """Plate template content."""

    @property
    def example(self) -> str | None:
        """Example markdown content."""

    @property
    def metadata(self) -> dict[str, Any]:
        """Quill metadata."""

    @property
    def schema(self) -> Any:
        """Field schema definitions."""

    @property
    def defaults(self) -> dict[str, Any]:
        """Default field values from schema."""

    @property
    def examples(self) -> dict[str, list[Any]]:
        """Example field values from schema."""

    @property
    def print_tree(self) -> str:
        """Get file tree representation."""

    def supported_formats(self) -> list[OutputFormat]:
        """Get supported output formats for this quill's backend."""
```

**Example:**

```python
quill = Quill.from_path("my-quill/")

print(f"Name: {quill.name}")
print(f"Backend: {quill.backend}")
print(f"Schema: {quill.schema}")

# Print file structure
print(quill.print_tree())
```

### ParsedDocument

Parsed markdown with frontmatter.

```python
class ParsedDocument:
    @staticmethod
    def from_markdown(markdown: str) -> ParsedDocument:
        """Parse markdown with frontmatter.

        Args:
            markdown: Markdown content

        Returns:
            Parsed document

        Raises:
            ParseError: If YAML invalid
        """

    def body(self) -> str | None:
        """Get document body (stored in BODY field)."""

    def get_field(self, key: str) -> Any | None:
        """Get frontmatter field by key (e.g., 'title', 'BODY', 'CARDS')."""

    @property
    def fields(self) -> dict[str, Any]:
        """All frontmatter fields (includes BODY and CARDS)."""

    def quill_name(self) -> str:
        """Get QUILL field value from the document."""
```

**Example:**

```python
markdown = """---
title: My Document
author: John Doe
tags: [python, tutorial]
---

# Introduction

Content here.
"""

parsed = ParsedDocument.from_markdown(markdown)

print(parsed.get_field("title"))  # "My Document"
print(parsed.get_field("tags"))   # ["python", "tutorial"]
print(parsed.body())               # "# Introduction\n\nContent here."
print(parsed.get_field("BODY"))   # Same as body()
print(parsed.fields)               # Complete field dict (includes BODY)
```

### RenderResult

Result of rendering operation.

```python
class RenderResult:
    @property
    def artifacts(self) -> list[Artifact]:
        """Output artifacts."""

    @property
    def warnings(self) -> list[Diagnostic]:
        """Warning diagnostics."""

    @property
    def output_format(self) -> OutputFormat:
        """Output format that was produced."""
```

**Example:**

```python
result = workflow.render(parsed, OutputFormat.PDF)

print(f"Format: {result.output_format}")  # OutputFormat.PDF
print(f"Artifacts: {len(result.artifacts)}")
for artifact in result.artifacts:
    print(f"  Format: {artifact.output_format}")
    print(f"  Size: {len(artifact.bytes)} bytes")

if result.warnings:
    for warning in result.warnings:
        print(f"Warning: {warning.message}")
```

### Artifact

Output artifact (PDF, SVG, etc.).

```python
class Artifact:
    @property
    def bytes(self) -> bytes:
        """Artifact binary data."""

    @property
    def output_format(self) -> OutputFormat:
        """Output format."""

    @property
    def mime_type(self) -> str:
        """MIME type of the artifact.

        Returns:
            'application/pdf' for PDF
            'image/svg+xml' for SVG
            'text/plain' for TXT
        """

    def save(self, path: str | Path) -> None:
        """Save to file."""
```

**Example:**

```python
artifact = result.artifacts[0]

# Check format and mime type
print(artifact.output_format)  # OutputFormat.PDF
print(artifact.mime_type)      # 'application/pdf'

# Save to file
artifact.save("output.pdf")

# Or access bytes directly
pdf_data = artifact.bytes
with open("output.pdf", "wb") as f:
    f.write(pdf_data)
```

## Enums

### OutputFormat

Output format enumeration.

```python
class OutputFormat:
    PDF = "pdf"
    SVG = "svg"
    TXT = "txt"
```

**Example:**

```python
from quillmark import OutputFormat

# Use in render calls
result = workflow.render(parsed, OutputFormat.PDF)
result = workflow.render(parsed, OutputFormat.SVG)
```

### Severity

Diagnostic severity levels.

```python
class Severity:
    ERROR = "error"
    WARNING = "warning"
    NOTE = "note"
```

## Diagnostics

### Location

Source code location information.

```python
class Location:
    @property
    def file(self) -> str:
        """Source file path."""

    @property
    def line(self) -> int:
        """Line number."""

    @property
    def col(self) -> int:
        """Column number."""
```

### Diagnostic

Error or warning diagnostic.

```python
class Diagnostic:
    @property
    def severity(self) -> Severity:
        """Diagnostic severity."""

    @property
    def message(self) -> str:
        """Diagnostic message."""

    @property
    def code(self) -> str | None:
        """Error code."""

    @property
    def primary(self) -> Location | None:
        """Primary location."""

    @property
    def hint(self) -> str | None:
        """Helpful hint."""

    @property
    def source_chain(self) -> list[str]:
        """Chain of error sources."""
```

## Exceptions

### QuillmarkError

Base exception for all Quillmark errors.

```python
class QuillmarkError(Exception):
    """Base exception for Quillmark errors."""
```

### ParseError

YAML parsing failed.

```python
class ParseError(QuillmarkError):
    """YAML parsing failed."""
```

### TemplateError

Template rendering failed.

```python
class TemplateError(QuillmarkError):
    """Template rendering failed."""
```

### CompilationError

Backend compilation failed.

```python
class CompilationError(QuillmarkError):
    """Backend compilation failed."""
```

**Example Error Handling:**

```python
from quillmark import ParseError, QuillmarkError

try:
    parsed = ParsedDocument.from_markdown(markdown)
    result = workflow.render(parsed, OutputFormat.PDF)
except ParseError as e:
    print(f"Parse error: {e}")
except QuillmarkError as e:
    print(f"Quillmark error: {e}")
```

## Complete Example

```python
from pathlib import Path
from quillmark import (
    Quillmark,
    Quill,
    ParsedDocument,
    OutputFormat,
    QuillmarkError
)

def render_document(quill_path: str, markdown: str, output_path: str):
    """Render a markdown document using a quill template."""
    try:
        # Setup engine
        engine = Quillmark()

        # Load quill
        quill = Quill.from_path(quill_path)
        engine.register_quill(quill)

        # Parse markdown
        parsed = ParsedDocument.from_markdown(markdown)

        # Create workflow
        workflow = engine.workflow(quill.name)

        # Validate first (fast)
        workflow.dry_run(parsed)

        # Check supported formats
        formats = workflow.supported_formats
        print(f"Supported formats: {formats}")

        # Render
        if OutputFormat.PDF in formats:
            result = workflow.render(parsed, OutputFormat.PDF)
        else:
            result = workflow.render(parsed, formats[0])

        # Save first artifact
        result.artifacts[0].save(output_path)
        print(f"Saved to: {output_path}")

        # Report warnings
        if result.warnings:
            for warning in result.warnings:
                print(f"Warning: {warning.message}")

        return True

    except QuillmarkError as e:
        print(f"Error: {e}")
        return False

# Usage
markdown_content = """---
title: My Document
author: Alice
---

# Hello World

This is my document.
"""

render_document(
    quill_path="my-quill/",
    markdown=markdown_content,
    output_path="output.pdf"
)
```

## Next Steps

- [Quickstart Guide](../getting-started/quickstart.md)
- [Creating Quills](../guides/creating-quills.md)
- [Rust API Documentation](https://docs.rs/quillmark/latest/quillmark/)
