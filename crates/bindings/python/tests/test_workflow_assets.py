"""Tests for workflow dynamic assets and fonts."""

import pytest

from quillmark import OutputFormat, Document, Quill, Quillmark, QuillmarkError


def test_render_result_output_format(taro_quill_dir, taro_md):
    """Test that RenderResult exposes output_format property."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)
    parsed = Document.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)
    assert result.output_format == OutputFormat.PDF


def test_artifact_mime_type(taro_quill_dir, taro_md):
    """Test that Artifact exposes mime_type property."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)
    parsed = Document.from_markdown(taro_md)

    result_pdf = workflow.render(parsed, OutputFormat.PDF)
    assert len(result_pdf.artifacts) > 0
    assert result_pdf.artifacts[0].mime_type == "application/pdf"

    result_svg = workflow.render(parsed, OutputFormat.SVG)
    assert len(result_svg.artifacts) > 0
    assert result_svg.artifacts[0].mime_type == "image/svg+xml"


def test_add_asset_collision(taro_quill_dir):
    """Test that adding duplicate asset raises error."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)
    workflow.add_asset("test.png", b"data1")
    with pytest.raises(QuillmarkError):
        workflow.add_asset("test.png", b"data2")


def test_add_font_collision(taro_quill_dir):
    """Test that adding duplicate font raises error."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)
    workflow.add_font("custom.ttf", b"font1")
    with pytest.raises(QuillmarkError):
        workflow.add_font("custom.ttf", b"font2")


def test_dynamic_asset_names_empty(taro_quill_dir):
    """Test dynamic_asset_names returns empty list initially."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)
    assert workflow.dynamic_asset_names() == []


def test_dynamic_font_names_empty(taro_quill_dir):
    """Test dynamic_font_names returns empty list initially."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)
    assert workflow.dynamic_font_names() == []
