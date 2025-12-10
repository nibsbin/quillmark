"""Tests for rendering workflow."""

import pytest

from quillmark import OutputFormat, ParsedDocument, Quill, Quillmark


def test_end_to_end_render(taro_quill_dir, taro_md):
    """Test end-to-end rendering."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow("taro")
    parsed = ParsedDocument.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)
    
    assert len(result.artifacts) == 1
    assert result.artifacts[0].output_format == OutputFormat.PDF
    assert len(result.artifacts[0].bytes) > 0


def test_process_plate(taro_quill_dir, taro_md):
    """Test processing plate template."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow("taro")
    parsed = ParsedDocument.from_markdown(taro_md)
    plated = workflow.process_plate(parsed)
    
    assert "nutty" in plated
    assert "earthy" in plated


def test_save_artifact(taro_quill_dir, taro_md, tmp_path):
    """Test saving an artifact to file."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow(quill)
    parsed = ParsedDocument.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)
    
    output_path = tmp_path / "output.pdf"
    result.artifacts[0].save(str(output_path))
    
    assert output_path.exists()
    assert output_path.stat().st_size > 0
