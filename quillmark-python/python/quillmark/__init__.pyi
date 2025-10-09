"""Type stubs for quillmark."""

from pathlib import Path
from typing import Any

class OutputFormat:
    PDF: OutputFormat
    SVG: OutputFormat
    TXT: OutputFormat

class Severity:
    ERROR: Severity
    WARNING: Severity
    NOTE: Severity

class Location:
    @property
    def file(self) -> str | None: ...
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
    
    def register_quill(self, quill: Quill) -> None:
        """Register a quill template with the engine."""
    
    def workflow_from_quill_name(self, name: str) -> Workflow:
        """Load workflow by quill name (must be registered).
        
        Raises:
            QuillmarkError: If quill is not registered or backend unavailable
        """
    
    def workflow_from_quill(self, quill: Quill) -> Workflow:
        """Load workflow from quill object (doesn't need to be registered)."""
    
    def workflow_from_parsed(self, parsed: ParsedDocument) -> Workflow:
        """Load workflow from parsed document with QUILL field.
        
        Raises:
            QuillmarkError: If document lacks QUILL field
        """
    
    def registered_backends(self) -> list[str]:
        """Get list of registered backend IDs."""
    
    def registered_quills(self) -> list[str]:
        """Get list of registered quill names."""

class Workflow:
    """Sealed workflow for executing the render pipeline."""
    
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
    
    def render_source(
        self,
        content: str,
        format: OutputFormat | None = None
    ) -> RenderResult:
        """Render pre-composed content (skip template processing)."""
    
    def process_glue(self, markdown: str) -> str:
        """Process markdown through glue template, return composed output."""
    
    def process_glue_parsed(self, parsed: ParsedDocument) -> str:
        """Process parsed document through glue template."""
    
    def with_asset(self, filename: str, contents: bytes) -> Workflow:
        """Add dynamic asset (returns new workflow instance)."""
    
    def with_assets(self, assets: dict[str, bytes]) -> Workflow:
        """Add multiple dynamic assets."""
    
    def clear_assets(self) -> Workflow:
        """Remove all dynamic assets."""
    
    def with_font(self, filename: str, contents: bytes) -> Workflow:
        """Add dynamic font."""
    
    def with_fonts(self, fonts: dict[str, bytes]) -> Workflow:
        """Add multiple dynamic fonts."""
    
    def clear_fonts(self) -> Workflow:
        """Remove all dynamic fonts."""
    
    def backend_id(self) -> str:
        """Get backend identifier."""
    
    def supported_formats(self) -> list[OutputFormat]:
        """Get supported output formats."""
    
    def quill_name(self) -> str:
        """Get quill name."""
    
    def dynamic_asset_names(self) -> list[str]:
        """Get list of dynamic asset filenames."""
    
    def dynamic_font_names(self) -> list[str]:
        """Get list of dynamic font filenames."""

class Quill:
    """Template bundle containing glue templates and assets."""
    
    @staticmethod
    def from_path(path: str | Path) -> Quill:
        """Load quill from filesystem path.
        
        Raises:
            QuillmarkError: If path doesn't exist or quill is invalid
        """
    
    @property
    def name(self) -> str:
        """Quill name from Quill.toml"""
    
    @property
    def backend(self) -> str | None:
        """Backend identifier from metadata"""
    
    @property
    def glue_template(self) -> str:
        """Glue template content"""

    @property
    def example(self) -> str | None:
        """Optional example template filename/content declared by the quill."""
    
    @property
    def metadata(self) -> dict[str, Any]:
        """Quill metadata from Quill.toml"""

    @property
    def field_schemas(self) -> dict[str, Any]:
        """Field schema definitions declared by the quill (from Quill.toml)."""

class ParsedDocument:
    """Parsed markdown document with frontmatter."""
    
    @staticmethod
    def from_markdown(markdown: str) -> ParsedDocument:
        """Parse markdown with YAML frontmatter.
        
        Raises:
            ParseError: If YAML frontmatter is invalid
        """
    
    def body(self) -> str:
        """Get document body content."""
    
    def get_field(self, key: str) -> Any | None:
        """Get frontmatter field value."""
    
    def fields(self) -> dict[str, Any]:
        """Get all frontmatter fields."""
    
    def quill_tag(self) -> str | None:
        """Get QUILL field value if present."""

class RenderResult:
    """Result of rendering operation."""
    
    @property
    def artifacts(self) -> list[Artifact]:
        """Output artifacts"""
    
    @property
    def warnings(self) -> list[Diagnostic]:
        """Warning diagnostics"""

class Artifact:
    """Output artifact (PDF, SVG, etc.)."""
    
    @property
    def bytes(self) -> bytes:
        """Artifact binary data"""
    
    @property
    def output_format(self) -> OutputFormat:
        """Output format"""
    
    def save(self, path: str | Path) -> None:
        """Save artifact to file."""
