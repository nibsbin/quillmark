"""Tests for the API requirements."""

import pytest
from quillmark import Quillmark, ParsedDocument, OutputFormat, ParseError
from conftest import QUILLS_PATH, _latest_version


def test_parsed_document_quill_ref():
    """Test that ParsedDocument exposes quill_ref method."""
    markdown_with_quill = "---\nQUILL: my_quill\ntitle: Test\n---\n\n# Content\n"
    parsed = ParsedDocument.from_markdown(markdown_with_quill)
    assert parsed.quill_ref() == "my_quill"

    markdown_without_quill = "---\ntitle: Test\n---\n\n# Content\n"
    with pytest.raises(ParseError):
        ParsedDocument.from_markdown(markdown_without_quill)


def test_quill_properties(taro_quill_dir):
    """Test that Quill exposes all required properties."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))

    assert quill.name == "taro"
    assert quill.backend == "typst"
    assert quill.plate is not None
    assert isinstance(quill.plate, str)

    metadata = quill.metadata
    assert isinstance(metadata, dict)

    schema = quill.schema
    assert isinstance(schema, str)
    assert "fields:" in schema

    example = quill.example
    assert example is not None

    supported_formats = quill.supported_formats()
    assert isinstance(supported_formats, list)
    assert OutputFormat.PDF in supported_formats


def test_full_workflow():
    """Test loading quill via engine and rendering."""
    engine = Quillmark()
    taro_dir = QUILLS_PATH / "taro"
    quill = engine.quill_from_path(str(_latest_version(taro_dir)))
    workflow = engine.workflow(quill)

    markdown = "---\nQUILL: taro\nauthor: Test Author\nice_cream: Chocolate\ntitle: Test\n---\n\nContent.\n"
    parsed = ParsedDocument.from_markdown(markdown)
    assert parsed.quill_ref() == "taro"

    assert "taro" in workflow.quill_ref
    assert workflow.backend_id == "typst"
    assert OutputFormat.PDF in workflow.supported_formats

    result = workflow.render(parsed, OutputFormat.PDF)
    assert len(result.artifacts) > 0
    assert result.artifacts[0].output_format == OutputFormat.PDF
    assert len(result.artifacts[0].bytes) > 0
