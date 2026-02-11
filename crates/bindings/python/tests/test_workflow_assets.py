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


def test_add_asset(taro_quill_dir, taro_md):
    """Test adding a dynamic asset to workflow."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add a test asset
    test_data = b"test image data"
    workflow.add_asset("test.png", test_data)

    # Verify it was added
    asset_names = workflow.dynamic_asset_names()
    assert "test.png" in asset_names


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


def test_add_assets(taro_quill_dir):
    """Test adding multiple assets at once."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add multiple assets
    assets = [
        ("image1.png", b"image data 1"),
        ("image2.png", b"image data 2"),
    ]
    workflow.add_assets(assets)

    # Verify both were added
    asset_names = workflow.dynamic_asset_names()
    assert "image1.png" in asset_names
    assert "image2.png" in asset_names


def test_clear_assets(taro_quill_dir):
    """Test clearing all dynamic assets."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add assets
    workflow.add_asset("test1.png", b"data1")
    workflow.add_asset("test2.png", b"data2")
    assert len(workflow.dynamic_asset_names()) == 2

    # Clear assets
    workflow.clear_assets()
    assert len(workflow.dynamic_asset_names()) == 0


def test_add_font(taro_quill_dir):
    """Test adding a dynamic font to workflow."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add a test font
    test_font = b"fake font data"
    workflow.add_font("custom.ttf", test_font)

    # Verify it was added
    font_names = workflow.dynamic_font_names()
    assert "custom.ttf" in font_names


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


def test_add_fonts(taro_quill_dir):
    """Test adding multiple fonts at once."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add multiple fonts
    fonts = [
        ("font1.ttf", b"font data 1"),
        ("font2.otf", b"font data 2"),
    ]
    workflow.add_fonts(fonts)

    # Verify both were added
    font_names = workflow.dynamic_font_names()
    assert "font1.ttf" in font_names
    assert "font2.otf" in font_names


def test_clear_fonts(taro_quill_dir):
    """Test clearing all dynamic fonts."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)

    workflow = engine.workflow("taro")

    # Add fonts
    workflow.add_font("font1.ttf", b"font1")
    workflow.add_font("font2.otf", b"font2")
    assert len(workflow.dynamic_font_names()) == 2

    # Clear fonts
    workflow.clear_fonts()
    assert len(workflow.dynamic_font_names()) == 0


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
