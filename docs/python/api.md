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
result = workflow.render(parsed, OutputFormat.PDF)

# Save output
with open("output.pdf", "wb") as f:
    f.write(result.artifacts[0].bytes)
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
    
    def workflow(self, name: str) -> Workflow:
        """Get workflow by quill name.
        
        Args:
            name: Registered quill name
            
        Returns:
            Workflow for the quill
            
        Raises:
            QuillmarkError: If quill not registered
        """
    
    def workflow_from_quill(self, quill: Quill) -> Workflow:
        """Get workflow from quill object."""
    
    def workflow(self, parsed: ParsedDocument) -> Workflow:
        """Get workflow from document's QUILL field.
        
        Raises:
            QuillmarkError: If QUILL field missing
        """
    
    def registered_backends(self) -> list[str]:
        """Get registered backend IDs."""
    
    def registered_quills(self) -> list[str]:
        """Get registered quill names."""

    def unregister_quill(self, name: str) -> None:
        """Unregister a quill by name.

        Args:
            name: Quill name to unregister

        Raises:
            QuillmarkError: If quill not registered
        """

    def get_quill(self, name: str) -> Quill:
        """Get registered quill by name.

        Args:
            name: Registered quill name

        Returns:
            The registered quill

        Raises:
            QuillmarkError: If quill not registered
        """

    def get_quill_metadata(self, name: str) -> dict[str, Any]:
        """Get quill metadata by name.

        Args:
            name: Registered quill name

        Returns:
            Quill metadata dictionary

        Raises:
            QuillmarkError: If quill not registered
        """
```

**Example:**

```python
engine = Quillmark()

# Check available backends
print(engine.registered_backends())  # ['typst', 'acroform']

# Load and register quill
from quillmark import Quill
quill = Quill.from_path("my-quill/")
engine.register_quill(quill)

# Check registered quills
print(engine.registered_quills())  # ['my-quill', '__default__']
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
            TemplateError: Template composition failed
            CompilationError: Backend compilation failed
        """
    
    def render_processed(
        self,
        content: str,
        format: OutputFormat | None = None
    ) -> RenderResult:
        """Render pre-composed content."""
    
    def process_plate(self, parsed: ParsedDocument) -> str:
        """Process plate template without compilation.

        Args:
            parsed: Parsed markdown document

        Returns:
            Rendered template content
        """

    def dry_run(self, parsed: ParsedDocument) -> None:
        """Validate document without compilation.

        Args:
            parsed: Parsed markdown document

        Raises:
            TemplateError: If template validation fails
        """

    def validate_schema(self, parsed: ParsedDocument) -> None:
        """Validate document against quill schema.

        Args:
            parsed: Parsed markdown document

        Raises:
            QuillmarkError: If schema validation fails
        """

    def add_asset(self, name: str, data: bytes) -> None:
        """Add runtime asset to workflow.

        Args:
            name: Asset identifier (e.g., 'logo.png')
            data: Asset binary data
        """

    def add_assets(self, assets: dict[str, bytes]) -> None:
        """Add multiple runtime assets.

        Args:
            assets: Dict mapping asset names to binary data
        """

    def add_font(self, name: str, data: bytes) -> None:
        """Add runtime font to workflow.

        Args:
            name: Font identifier (e.g., 'custom.ttf')
            data: Font binary data
        """

    def add_fonts(self, fonts: dict[str, bytes]) -> None:
        """Add multiple runtime fonts.

        Args:
            fonts: Dict mapping font names to binary data
        """

    def dynamic_asset_names(self) -> list[str]:
        """Get names of dynamically added assets."""

    def dynamic_font_names(self) -> list[str]:
        """Get names of dynamically added fonts."""

    @property
    def backend_id(self) -> str:
        """Backend identifier."""
    
    @property
    def supported_formats(self) -> list[OutputFormat]:
        """Supported output formats."""
    
    @property
    def quill_name(self) -> str:
        """Quill name."""
```

**Example:**

```python
workflow = engine.workflow("my-quill")

print(f"Backend: {workflow.backend_id}")  # 'typst'
print(f"Formats: {workflow.supported_formats}")  # [PDF, SVG]

# Render to specific format
result = workflow.render(parsed, OutputFormat.PDF)
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

    @staticmethod
    def from_json(json_str: str) -> Quill:
        """Load quill from JSON configuration.

        Args:
            json_str: JSON string containing quill config

        Returns:
            Loaded quill

        Raises:
            QuillmarkError: If JSON invalid or quill malformed
        """

    @property
    def name(self) -> str:
        """Quill name from Quill.toml."""
    
    @property
    def backend(self) -> str | None:
        """Backend identifier."""
    
    @property
    def plate(self) -> str:
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
    
    def print_tree(self) -> str:
        """Get file tree representation."""
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

    def quill_tag(self) -> str:
        """Get QUILL field value (defaults to '__default__' if not specified)."""
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
```

**Example:**

```python
result = workflow.render(parsed, OutputFormat.PDF)

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
    
    def save(self, path: str | Path) -> None:
        """Save to file."""
```

**Example:**

```python
artifact = result.artifacts[0]

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
    def file(self) -> str | None:
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
from quillmark import ParseError, TemplateError, CompilationError

try:
    parsed = ParsedDocument.from_markdown(markdown)
    result = workflow.render(parsed, OutputFormat.PDF)
except ParseError as e:
    print(f"Parse error: {e}")
except TemplateError as e:
    print(f"Template error: {e}")
except CompilationError as e:
    print(f"Compilation error: {e}")
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
