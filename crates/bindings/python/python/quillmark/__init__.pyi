"""Type stubs for quillmark."""

from pathlib import Path
from typing import Any
from enum import Enum

class _OutputFormat:
    PDF = "pdf"
    SVG = "svg"
    TXT = "txt"

class _Severity:
    ERROR = "error"
    WARNING =  "warning"
    NOTE = "note"


class Location:
    @property
    def file(self) -> str: ...
    @property
    def line(self) -> int: ...
    @property
    def col(self) -> int: ...

class Diagnostic:
    @property
    def severity(self) -> Severity: ...
    @property
    def message(self) -> str: ...
    @property
    def code(self) -> str | None: ...
    @property
    def primary(self) -> Location | None: ...
    @property
    def hint(self) -> str | None: ...
    @property
    def source_chain(self) -> list[str]: ...

class QuillmarkError(Exception):
    """Base exception for Quillmark errors."""

class ParseError(QuillmarkError):
    """YAML parsing failed."""

class TemplateError(QuillmarkError):
    """Template rendering failed."""

class CompilationError(QuillmarkError):
    """Backend compilation failed."""

class Quillmark:
    """High-level engine for orchestrating backends and quills."""
    
    def __init__(self) -> None:
        """Create engine with auto-registered backends based on enabled features."""
    
    def quill_from_path(self, path: str | Path) -> Quill:
        """Load a quill from a filesystem path and attach the appropriate backend.
        
        Raises:
            QuillmarkError: If path doesn't exist, quill is invalid, or backend unavailable
        """

    def workflow(self, quill: Quill) -> Workflow:
        """Create a workflow for the given quill.

        Args:
            quill: A Quill object (must have been loaded via quill_from_path or load_quill)

        Raises:
            QuillmarkError: If backend unavailable
        """
    
    def registered_backends(self) -> list[str]:
        """Get list of registered backend IDs."""

class Workflow:
    """Sealed workflow for executing the render pipeline.
    
    Supports dynamic asset and font injection at runtime via add_asset/add_font methods.
    """
    
    def render(
        self,
        parsed: ParsedDocument,
        format: OutputFormat | None = None
    ) -> RenderResult:
        """Render parsed document to artifacts."""

    def dry_run(self, parsed: ParsedDocument) -> None:
        """Validate document without compilation."""

    @property
    def backend_id(self) -> str:
        """Get backend identifier."""
    
    @property
    def supported_formats(self) -> list[OutputFormat]:
        """Get supported output formats."""
    
    @property
    def quill_ref(self) -> str:
        """Get quill reference (name@version)."""

    def add_asset(self, filename: str, contents: bytes) -> None:
        """Add a dynamic asset to the workflow."""

    def add_assets(self, assets: list[tuple[str, bytes]]) -> None:
        """Add multiple dynamic assets at once."""

    def clear_assets(self) -> None:
        """Clear all dynamic assets from the workflow."""

    def dynamic_asset_names(self) -> list[str]:
        """Get list of dynamic asset filenames currently in the workflow."""

    def add_font(self, filename: str, contents: bytes) -> None:
        """Add a dynamic font to the workflow."""

    def add_fonts(self, fonts: list[tuple[str, bytes]]) -> None:
        """Add multiple dynamic fonts at once."""

    def clear_fonts(self) -> None:
        """Clear all dynamic fonts from the workflow."""

    def dynamic_font_names(self) -> list[str]:
        """Get list of dynamic font filenames currently in the workflow."""

class Quill:
    """Format bundle containing plate content and assets."""
    
    @staticmethod
    def from_path(path: str | Path) -> Quill:
        """Load quill from filesystem path (no backend attached).
        
        Raises:
            QuillmarkError: If path doesn't exist or quill is invalid
        """
    
    @property
    def name(self) -> str:
        """Quill name from Quill.yaml"""
    
    @property
    def backend(self) -> str:
        """Backend identifier"""

    @property
    def plate(self) -> str | None:
        """Plate template content"""

    @property
    def example(self) -> str | None:
        """Optional example template content"""
    
    @property
    def metadata(self) -> dict[str, Any]:
        """Quill metadata from Quill.yaml"""

    @property
    def schema(self) -> str:
        """Public quill schema as YAML text."""

    @property
    def defaults(self) -> dict[str, Any]:
        """Default field values extracted from schema."""

    @property
    def examples(self) -> dict[str, list[Any]]:
        """Example field values extracted from schema."""

    @property
    def print_tree(self) -> str:
        """Get a string representation of the quill file tree."""

    def supported_formats(self) -> list[OutputFormat]:
        """Get supported output formats for this quill's backend."""

    def render(
        self,
        input: str | ParsedDocument,
        format: OutputFormat | None = None,
    ) -> RenderResult:
        """Render a document using this quill.

        For dynamic asset or font injection, use engine.workflow(quill) instead.

        Args:
            input: Markdown string or pre-parsed ParsedDocument
            format: Output format (defaults to first supported format)

        Raises:
            QuillmarkError: If this quill was not loaded via engine.quill_from_path()
            TypeError: If input is not str or ParsedDocument
        """

class ParsedDocument:
    """Parsed markdown document with frontmatter."""
    
    @staticmethod
    def from_markdown(markdown: str) -> ParsedDocument:
        """Parse markdown with YAML frontmatter.
        
        Raises:
            ParseError: If YAML frontmatter is invalid or QUILL is missing
        """
    
    def body(self) -> str | None:
        """Get document body content."""
    
    def get_field(self, key: str) -> Any | None:
        """Get frontmatter field value."""
    
    @property
    def fields(self) -> dict[str, Any]:
        """Get all frontmatter fields."""

    def quill_ref(self) -> str:
        """Get quill reference from the document."""

class RenderResult:
    """Result of rendering operation."""
    
    @property
    def artifacts(self) -> list[Artifact]:
        """Output artifacts"""
    
    @property
    def warnings(self) -> list[Diagnostic]:
        """Warning diagnostics"""

    @property
    def output_format(self) -> OutputFormat:
        """Output format that was produced"""

class Artifact:
    """Output artifact (PDF, SVG, etc.)."""
    
    @property
    def bytes(self) -> bytes:
        """Artifact binary data"""
    
    @property
    def output_format(self) -> OutputFormat:
        """Output format"""

    @property
    def mime_type(self) -> str:
        """MIME type of the artifact"""
    
    def save(self, path: str | Path) -> None:
        """Save artifact to file."""
