"""Type stubs for the compiled `quillmark._quillmark` extension.

The Tier-1 surface `prose/canon/BINDINGS.md` freezes, spelled for type checkers:
pyo3 docstrings are runtime-only, so without these the whole API resolves to
`Any`. Each declaration mirrors its `#[pymethods]` twin in `src/types.rs` /
`src/enums.rs`; the docstrings there are the long form, these the one-line
summaries an IDE shows. The drift guard is
`python -m mypy.stubtest --ignore-disjoint-bases quillmark`, run in CI against
the built module — a signature that diverges from `src/` fails there.

A `Card` crosses as a plain dict — `{kind, quill, id, payload_items, ext, seed,
body}` — and a payload value as whatever JSON maps to, so both are `Any`-shaped
rather than protocol-typed: their schemas are the quill's, not Python's.
"""

from pathlib import Path
from typing import Any, final

__all__ = [
    "Artifact",
    "CardReader",
    "CardWriter",
    "Diagnostic",
    "Document",
    "Location",
    "OutputFormat",
    "Quill",
    "Quillmark",
    "QuillmarkError",
    "Reader",
    "RenderResult",
    "Severity",
    "Writer",
]

@final
class QuillmarkError(Exception):
    """The one raised exception; always carries a non-empty `diagnostics` list."""

    diagnostics: list[Diagnostic]

@final
class Severity:
    """Diagnostic level."""

    ERROR: Severity
    WARNING: Severity

    @property
    def name(self) -> str: ...
    @staticmethod
    def all() -> list[Severity]: ...
    def __repr__(self) -> str: ...

@final
class OutputFormat:
    """A format a backend can emit."""

    PDF: OutputFormat
    SVG: OutputFormat
    PNG: OutputFormat

    @property
    def name(self) -> str: ...
    @staticmethod
    def all() -> list[OutputFormat]: ...
    def __repr__(self) -> str: ...

@final
class Location:
    """A file position on a diagnostic."""

    @property
    def file(self) -> str: ...
    @property
    def line(self) -> int: ...
    @property
    def column(self) -> int: ...

@final
class Diagnostic:
    """One diagnostic. Route on `code`, not on message text."""

    @property
    def severity(self) -> Severity: ...
    @property
    def message(self) -> str: ...
    @property
    def code(self) -> str | None: ...
    @property
    def location(self) -> Location | None: ...
    @property
    def hint(self) -> str | None: ...
    @property
    def path(self) -> str | None: ...
    @property
    def source_chain(self) -> list[str]: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@final
class Artifact:
    """One rendered output — bytes plus the format that produced them."""

    @property
    def bytes(self) -> bytes: ...
    @property
    def format(self) -> OutputFormat: ...
    @property
    def mime_type(self) -> str: ...
    def save(self, path: str) -> None: ...

@final
class RenderResult:
    """What one `Quillmark.render` produced."""

    @property
    def artifacts(self) -> list[Artifact]: ...
    @property
    def warnings(self) -> list[Diagnostic]: ...
    @property
    def format(self) -> OutputFormat: ...
    @property
    def render_time_ms(self) -> float: ...
    @property
    def regions(self) -> list[dict[str, Any]]:
        """Field-geometry sidecar, populated only by `render(..., regions=True)`."""

@final
class Document:
    """Quill-free document data and structure — parse, the storage DTO, cards,
    `$ext` / `$seed`. Field I/O lives on `Quill.writer(doc)` / `Quill.reader(doc)`."""

    def __new__(cls, quill_ref: str) -> Document: ...
    @staticmethod
    def from_markdown(markdown: str) -> Document: ...
    @staticmethod
    def from_json(json: str) -> Document: ...
    @staticmethod
    def try_from_json(json: str) -> Document | None: ...
    @staticmethod
    def schema_version_of(json: str) -> str | None: ...
    @staticmethod
    def current_schema_version() -> str: ...
    @staticmethod
    def format_rules() -> str: ...
    @staticmethod
    def blueprint_instruction(quill_name: str) -> str: ...
    @staticmethod
    def quill_ref_hint() -> str: ...
    @staticmethod
    def make_card(
        kind: str, fields: dict[str, Any] | None = None, body: str | None = None
    ) -> dict[str, Any]:
        """A fresh card dict from a kind and a flat field mapping."""

    def to_markdown(self) -> str: ...
    def to_json(self) -> str: ...
    @property
    def quill_ref(self) -> str: ...
    def set_quill_ref(self, ref_str: str) -> None: ...
    def clone(self) -> Document: ...
    def equals(self, other: Document) -> bool: ...
    def __copy__(self) -> Document: ...
    def __deepcopy__(self, _memo: Any) -> Document: ...
    def __eq__(self, other: object, /) -> bool: ...
    def __repr__(self) -> str: ...
    @property
    def card_count(self) -> int: ...
    @property
    def warnings(self) -> list[Diagnostic]:
        """Parse warnings — session state, excluded from `equals` and the DTO."""

    @property
    def body(self) -> Any:
        """Main body as canonical Content-JSON."""

    @property
    def main(self) -> dict[str, Any]:
        """Main (entry) card dict."""

    @property
    def cards(self) -> list[dict[str, Any]]:
        """Ordered composable card dicts."""

    def card(self, index: int) -> dict[str, Any]:
        """One composable card, same dict shape as `main`; out of range raises."""

    def card_index_by_id(self, id: str) -> int | None:
        """The index of the card carrying `$id`, or `None`."""

    def seed_overlay(self, kind: str) -> Any:
        """The main card's `$seed[kind]` overlay dict, or `None`."""

    def remove_field(self, name: str, card: int | None = None) -> Any: ...
    def store_ext(self, value: Any, card: int | None = None) -> None: ...
    def remove_ext(self, card: int | None = None) -> Any: ...
    def store_ext_namespace(
        self, namespace: str, value: Any, card: int | None = None
    ) -> None: ...
    def remove_ext_namespace(self, namespace: str, card: int | None = None) -> Any: ...
    def store_seed_namespace(self, card_kind: str, overlay: Any) -> None: ...
    def remove_seed_namespace(self, card_kind: str) -> Any: ...
    def insert_card(self, card: Any, at: int | None = None) -> None: ...
    def remove_card(self, index: int) -> dict[str, Any] | None: ...
    def move_card(self, from_idx: int, to_idx: int) -> None: ...
    def set_card_kind(self, index: int, new_kind: str) -> None: ...

@final
class Quill:
    """Portable, declarative config data. The backend it declares is resolved at
    render time by a `Quillmark` engine, never here."""

    @staticmethod
    def from_path(path: str | Path) -> Quill: ...
    @property
    def backend_id(self) -> str: ...
    @property
    def quill_ref(self) -> str: ...
    @property
    def metadata(self) -> dict[str, Any]: ...
    @property
    def schema(self) -> Any: ...
    @property
    def blueprint(self) -> str: ...
    def writer(self, doc: Document) -> Writer:
        """Bind this quill's schema to `doc` for typed writes."""

    def reader(self, doc: Document) -> Reader:
        """Bind this quill's schema to `doc` for interpreted reads."""

    def validate(self, doc: Document) -> list[dict[str, Any]]:
        """Canonical `validation::*` diagnostic dicts; empty when valid."""

    def seed_document(self) -> Document: ...
    def seed_main(self) -> dict[str, Any]: ...
    def seed_card(
        self, card_kind: str, overlay: Any | None = None
    ) -> dict[str, Any] | None: ...

@final
class Writer:
    """A `Document` bound to its `Quill` for typed writes, from `Quill.writer(doc)`.
    Ephemeral by convention: bind, write, discard."""

    @property
    def document(self) -> Document: ...
    def set(self, name: str, value: Any) -> None: ...
    def set_all(self, fields: dict[str, Any]) -> None: ...
    def set_body(self, markdown: str) -> None: ...
    def revise_field(self, name: str, markdown: str) -> None:
        """Typed *and* anchor-preserving richtext write; the `Delta` is discarded."""

    def add_card(
        self,
        kind: str,
        fields: dict[str, Any] | None = None,
        body: str | None = None,
        at: int | None = None,
    ) -> None: ...
    def remove_card(self, index: int) -> dict[str, Any] | None: ...
    def card(self, index: int) -> CardWriter:
        """A cursor on the composable card at `index`; the index is checked at the write."""

@final
class CardWriter:
    """A composable card bound to its `Quill` for typed writes, from `Writer.card`."""

    @property
    def index(self) -> int: ...
    @property
    def kind(self) -> str | None: ...
    def set(self, name: str, value: Any) -> None: ...
    def set_all(self, fields: dict[str, Any]) -> None: ...
    def set_body(self, markdown: str) -> None: ...
    def revise_field(self, name: str, markdown: str) -> None: ...

@final
class Reader:
    """A `Document` bound to its `Quill` for interpreted reads, from
    `Quill.reader(doc)`. The field read surface — `Document` carries no quill-free
    field read. Ephemeral by convention: bind, read, discard."""

    @property
    def document(self) -> Document: ...
    def get(self, name: str) -> Any:
        """A main-card field read by its declared type; `None` when absent."""

    def get_body(self) -> str:
        """The main body's markdown — quill-free, never raising."""

    def card(self, index: int) -> CardReader: ...

@final
class CardReader:
    """A composable card bound to its `Quill` for interpreted reads, from `Reader.card`."""

    @property
    def index(self) -> int: ...
    @property
    def kind(self) -> str | None: ...
    def get(self, name: str) -> Any: ...
    def get_body(self) -> str: ...

@final
class Quillmark:
    """The render / capability dispatcher — the one object that resolves backends."""

    def __init__(self) -> None: ...
    def render(
        self,
        quill: Quill,
        doc: Document,
        format: OutputFormat | None = None,
        ppi: float | None = None,
        pages: list[int] | None = None,
        producer: str | None = None,
        regions: bool = False,
    ) -> RenderResult: ...
    def supported_formats(self, quill: Quill) -> list[OutputFormat]:
        """The formats `quill`'s backend can emit."""

    def registered_backends(self) -> list[str]:
        """The backend ids this build compiled in, in no guaranteed order."""
