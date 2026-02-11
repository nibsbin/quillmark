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
    
    def register_quill(self, quill: Quill) -> None:  # raises QuillmarkError on validation failure
        """Register a quill template with the engine."""
    
    def workflow(self, quill_ref: str | Quill | ParsedDocument) -> Workflow:
        """Load a workflow from a quill reference.

        Accepts:
            - str: Quill name (must be registered)
            - Quill: Quill object (doesn't need to be registered)
            - ParsedDocument: Parsed document (extracts QUILL field)

        Raises:
            QuillmarkError: If quill is not registered or backend unavailable
            TypeError: If quill_ref is not one of the accepted types
        """
    
    def registered_backends(self) -> list[str]:
        """Get list of registered backend IDs."""
    
    def registered_quills(self) -> list[str]:
        """Get list of registered quill names."""

class Workflow:
    """Sealed workflow for executing the render pipeline.
    
    Supports dynamic asset and font injection at runtime via add_asset/add_font methods.
    """
    
    def render(
        self,
        parsed: ParsedDocument,
        format: OutputFormat | None = None
    ) -> RenderResult:
        """Render parsed document to artifacts.
        
        Args:
            parsed: Parsed markdown document
            format: Output format (defaults to first supported format)
        
        Returns:
            RenderResult with artifacts and warnings
        
        Raises:
            TemplateError: If template composition fails
            CompilationError: If backend compilation fails
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
        """Get backend identifier."""
    
    @property
    def supported_formats(self) -> list[OutputFormat]:
        """Get supported output formats."""
    
    @property
    def quill_name(self) -> str:
        """Get quill name."""

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

class Quill:
    """Template bundle containing plate templates and assets."""
    
    @staticmethod
    def from_path(path: str | Path) -> Quill:
        """Load quill from filesystem path.
        
        Raises:
            QuillmarkError: If path doesn't exist or quill is invalid
        """
    
    @property
    def name(self) -> str:
        """Quill name from Quill.yaml"""
    
    @property
    def backend(self) -> str | None:
        """Backend identifier from metadata"""

    @property
    def plate(self) -> str | None:
        """Plate template content"""

    @property
    def example(self) -> str | None:
        """Optional example template filename/content declared by the quill."""
    
    @property
    def metadata(self) -> dict[str, Any]:
        """Quill metadata from Quill.yaml"""

    @property
    def schema(self) -> Any:
        """Field schema definitions declared by the quill (from Quill.yaml)."""

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

class ParsedDocument:
    """Parsed markdown document with frontmatter."""
    
    @staticmethod
    def from_markdown(markdown: str) -> ParsedDocument:
        """Parse markdown with YAML frontmatter.
        
        Raises:
            ParseError: If YAML frontmatter is invalid
        """
    
    def body(self) -> str | None:
        """Get document body content."""
    
    def get_field(self, key: str) -> Any | None:
        """Get frontmatter field value."""
    
    @property
    def fields(self) -> dict[str, Any]:
        """Get all frontmatter fields."""

    def quill_name(self) -> str:
        """Get QUILL field value from the document."""

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
        """MIME type of the artifact (e.g., 'application/pdf', 'image/svg+xml')"""
    
    def save(self, path: str | Path) -> None:
        """Save artifact to file."""
