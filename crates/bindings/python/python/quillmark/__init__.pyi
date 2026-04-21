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

class EditError(QuillmarkError):
    """Editor-surface invariant violated.

    Raised by ``Document`` and ``Card`` mutators.
    The exception message includes the variant name (e.g. ``ReservedName``,
    ``InvalidFieldName``, ``InvalidTagName``, ``IndexOutOfRange``) and details.
    """

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
            quill: A Quill object

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
        doc: Document,
        format: OutputFormat | None = None
    ) -> RenderResult:
        """Render document to artifacts."""

    def open(self, doc: Document) -> RenderSession:
        """Open an iterative render session for page-selective rendering."""

    def dry_run(self, doc: Document) -> None:
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
        doc: Document,
        format: OutputFormat | None = None,
    ) -> RenderResult:
        """Render a document using this quill.

        Args:
            doc: Pre-parsed Document
            format: Output format (defaults to first supported format)

        Raises:
            QuillmarkError: If rendering fails
        """

    def open(self, doc: Document) -> RenderSession:
        """Open an iterative render session for page-selective rendering."""

class Document:
    """Typed in-memory Quillmark document.

    Created via `Document.from_markdown(markdown)`.
    """

    @staticmethod
    def from_markdown(markdown: str) -> Document:
        """Parse Quillmark Markdown into a typed Document.

        Raises:
            ParseError: If YAML frontmatter is invalid or QUILL is missing
        """

    def to_markdown(self) -> str:
        """Emit canonical Quillmark Markdown.

        Not yet implemented — raises NotImplementedError until Phase 4.
        """

    def quill_ref(self) -> str:
        """The QUILL reference string (e.g. ``"usaf_memo@0.1"``)."""

    @property
    def frontmatter(self) -> dict[str, Any]:
        """Typed YAML frontmatter fields (no QUILL, BODY, or CARDS keys)."""

    @property
    def body(self) -> str:
        """Global Markdown body. Empty string when absent."""

    @property
    def cards(self) -> list[dict[str, Any]]:
        """Ordered list of card blocks.

        Each card dict has keys: ``tag`` (str), ``fields`` (dict), ``body`` (str).
        """

    @property
    def warnings(self) -> list[Diagnostic]:
        """Non-fatal parse-time warnings."""

    def set_field(self, name: str, value: Any) -> None:
        """Set a frontmatter field.

        Raises:
            EditError: If ``name`` is a reserved sentinel or does not match
                ``[a-z_][a-z0-9_]*``.
        """

    def remove_field(self, name: str) -> Any | None:
        """Remove a frontmatter field, returning the value or ``None``."""

    def set_quill_ref(self, ref_str: str) -> None:
        """Replace the QUILL reference string.

        Raises:
            ValueError: If ``ref_str`` is not a valid ``QuillReference``.
        """

    def replace_body(self, body: str) -> None:
        """Replace the global Markdown body."""

    def push_card(self, card: dict[str, Any]) -> None:
        """Append a card to the card list.

        ``card`` must be a dict with a ``tag`` key and optional ``fields`` and ``body``.

        Raises:
            EditError: If ``card["tag"]`` is not a valid tag name or a field name
                is invalid.
        """

    def insert_card(self, index: int, card: dict[str, Any]) -> None:
        """Insert a card at ``index``.

        Raises:
            EditError: If ``index`` is out of range or the card is invalid.
        """

    def remove_card(self, index: int) -> dict[str, Any] | None:
        """Remove and return the card at ``index``, or ``None`` if out of range."""

    def move_card(self, from_idx: int, to_idx: int) -> None:
        """Move the card at ``from_idx`` to ``to_idx``.

        ``from_idx == to_idx`` is a no-op.

        Raises:
            EditError: If either index is out of range.
        """

    def update_card_field(self, index: int, name: str, value: Any) -> None:
        """Update a field on the card at ``index``.

        Raises:
            EditError: If ``index`` is out of range or ``name`` is invalid.
        """

    def update_card_body(self, index: int, body: str) -> None:
        """Replace the body of the card at ``index``.

        Raises:
            EditError: If ``index`` is out of range.
        """

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

class RenderSession:
    @property
    def page_count(self) -> int: ...

    def render(
        self,
        format: OutputFormat | None = None,
        pages: list[int] | None = None,
    ) -> RenderResult: ...

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
