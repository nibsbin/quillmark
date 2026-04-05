"""Tests for workflow dynamic assets and fonts."""

import pytest

from quillmark import OutputFormat, ParsedDocument, Quill, Quillmark, QuillmarkError


def test_render_result_output_format(taro_quill_dir, taro_md):
    """Test that RenderResult exposes output_format property."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")
    parsed = ParsedDocument.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)

    # Test the new output_format property
    assert result.output_format == OutputFormat.PDF


def test_artifact_mime_type(taro_quill_dir, taro_md):
    """Test that Artifact exposes mime_type property."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")
    parsed = ParsedDocument.from_markdown(taro_md)

    # Test PDF mime type
    result_pdf = workflow.render(parsed, OutputFormat.PDF)
    assert len(result_pdf.artifacts) > 0
    assert result_pdf.artifacts[0].mime_type == "application/pdf"

    # Test SVG mime type
    result_svg = workflow.render(parsed, OutputFormat.SVG)
    assert len(result_svg.artifacts) > 0
    assert result_svg.artifacts[0].mime_type == "image/svg+xml"


def test_add_asset_collision(taro_quill_dir):
    """Test that adding duplicate asset raises error."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add an asset
    workflow.add_asset("test.png", b"data1")

    # Adding same filename should raise error
    with pytest.raises(QuillmarkError):
        workflow.add_asset("test.png", b"data2")


def test_add_font_collision(taro_quill_dir):
    """Test that adding duplicate font raises error."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add a font
    workflow.add_font("custom.ttf", b"font1")

    # Adding same filename should raise error
    with pytest.raises(QuillmarkError):
        workflow.add_font("custom.ttf", b"font2")


def test_dynamic_asset_names_empty(taro_quill_dir):
    """Test dynamic_asset_names returns empty list initially."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")
    assert workflow.dynamic_asset_names() == []


def test_dynamic_font_names_empty(taro_quill_dir):
    """Test dynamic_font_names returns empty list initially."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")
    assert workflow.dynamic_font_names() == []
