"""Tests for rendering workflow."""

import pytest

from quillmark import OutputFormat, ParsedDocument, Quill, Quillmark


def test_end_to_end_render(taro_quill_dir, taro_md):
    """Test end-to-end rendering."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow_from_quill_name("taro")
    parsed = ParsedDocument.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)
    
    assert len(result.artifacts) == 1
    assert result.artifacts[0].output_format == OutputFormat.PDF
    assert len(result.artifacts[0].bytes) > 0


def test_process_glue(taro_quill_dir, taro_md):
    """Test processing glue template."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow_from_quill_name("taro")
    parsed = ParsedDocument.from_markdown(taro_md)
    glue_output = workflow.process_glue_parsed(parsed)
    
    assert "nutty" in glue_output
    assert "earthy" in glue_output


def test_save_artifact(taro_quill_dir, taro_md, tmp_path):
    """Test saving an artifact to file."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow_from_quill(quill)
    parsed = ParsedDocument.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)
    
    output_path = tmp_path / "output.pdf"
    result.artifacts[0].save(str(output_path))
    
    assert output_path.exists()
    assert output_path.stat().st_size > 0
