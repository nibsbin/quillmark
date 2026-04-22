"""Smoke tests for quill.project_form (Phase 5).

NOTE: These tests cannot run in the devcontainer because the Python binding
is not built with `maturin develop` in this environment.  They are written
to run in CI where `maturin develop` (or `pip install -e .`) is available.

Expected environment: `quillmark` importable from a maturin-built wheel.
"""

import json
import pytest

try:
    from quillmark import Document, Quillmark
    QUILLMARK_AVAILABLE = True
except ImportError:
    QUILLMARK_AVAILABLE = False

pytestmark = pytest.mark.skipif(
    not QUILLMARK_AVAILABLE,
    reason="quillmark native module not available in this environment",
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

QUILL_YAML_CONTENT = """Quill:
  name: py_form_smoke
  version: "1.0"
  backend: typst
  description: Python project_form smoke test

main:
  fields:
    title:
      type: string
      default: Untitled
    count:
      type: integer
"""

MD_WITH_TITLE = "---\nQUILL: py_form_smoke\ntitle: \"Hello\"\n---\n"
MD_EMPTY = "---\nQUILL: py_form_smoke\n---\n"


def make_quill(tmp_path, yaml_content=QUILL_YAML_CONTENT):
    """Write a minimal quill directory and load it."""
    quill_dir = tmp_path / "quill"
    quill_dir.mkdir()
    (quill_dir / "Quill.yaml").write_text(yaml_content)
    engine = Quillmark()
    return engine.quill_from_path(quill_dir)


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

def test_project_form_returns_dict(tmp_path):
    """project_form returns a dict with main, cards, diagnostics."""
    quill = make_quill(tmp_path)
    doc = Document.from_markdown(MD_WITH_TITLE)

    projection = quill.project_form(doc)

    assert isinstance(projection, dict)
    assert "main" in projection
    assert "cards" in projection
    assert "diagnostics" in projection
    assert isinstance(projection["cards"], list)
    assert isinstance(projection["diagnostics"], list)


def test_project_form_document_source(tmp_path):
    """Fields present in the document get source='document'."""
    quill = make_quill(tmp_path)
    doc = Document.from_markdown(MD_WITH_TITLE)

    projection = quill.project_form(doc)
    values = projection["main"]["values"]

    assert "title" in values
    assert values["title"]["source"] == "document"
    assert values["title"]["value"] == "Hello"


def test_project_form_missing_source(tmp_path):
    """Fields absent from doc with no schema default get source='missing'."""
    quill = make_quill(tmp_path)
    doc = Document.from_markdown(MD_EMPTY)

    projection = quill.project_form(doc)
    values = projection["main"]["values"]

    # count has no default
    assert "count" in values
    assert values["count"]["source"] == "missing"
    assert values["count"]["value"] is None
    assert values["count"]["default"] is None


def test_project_form_default_source(tmp_path):
    """Fields absent from doc with a schema default get source='default'."""
    quill = make_quill(tmp_path)
    doc = Document.from_markdown(MD_EMPTY)

    projection = quill.project_form(doc)
    values = projection["main"]["values"]

    # title has default: "Untitled"
    assert "title" in values
    assert values["title"]["source"] == "default"
    assert values["title"]["value"] is None
    assert values["title"]["default"] == "Untitled"


def test_project_form_json_serializable(tmp_path):
    """FormProjection is fully JSON-serializable via json.dumps."""
    quill = make_quill(tmp_path)
    doc = Document.from_markdown(MD_WITH_TITLE)

    projection = quill.project_form(doc)
    dumped = json.dumps(projection)

    assert isinstance(dumped, str)
    assert len(dumped) > 0

    # Round-trip: parsed back is structurally identical
    parsed = json.loads(dumped)
    assert parsed["main"]["values"]["title"]["source"] == "document"


def test_project_form_unknown_card_diagnostic(tmp_path):
    """Unknown card tags produce a diagnostic and are excluded from cards."""
    quill = make_quill(tmp_path)
    md = (
        "---\nQUILL: py_form_smoke\ntitle: \"T\"\n---\n\n"
        "---\nCARD: ghost_card\nnote: \"B\"\n---\n"
    )
    doc = Document.from_markdown(md)

    projection = quill.project_form(doc)

    assert projection["cards"] == [], "unknown-tag card must be excluded"
    diag_codes = [d.get("code") for d in projection["diagnostics"]]
    assert "form::unknown_card_tag" in diag_codes, (
        f"expected form::unknown_card_tag diagnostic; got: {diag_codes}"
    )
