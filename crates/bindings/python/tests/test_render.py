"""Tests for rendering workflow."""

from quillmark import OutputFormat, ParsedDocument, Quillmark


def test_save_artifact(taro_quill_dir, taro_md, tmp_path):
    """Test saving an artifact to file."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)

    parsed = ParsedDocument.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)

    output_path = tmp_path / "output.pdf"
    result.artifacts[0].save(str(output_path))

    assert output_path.exists()
    assert output_path.stat().st_size > 0


def test_quill_render_from_markdown_string(taro_quill_dir, taro_md):
    """quill.render(str) parses internally and produces artifacts."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))

    result = quill.render(taro_md)

    assert len(result.artifacts) > 0
    assert len(result.artifacts[0].bytes) > 0


def test_quill_render_from_parsed_document(taro_quill_dir, taro_md):
    """quill.render(ParsedDocument) accepts a pre-parsed document."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    parsed = ParsedDocument.from_markdown(taro_md)

    result = quill.render(parsed)

    assert len(result.artifacts) > 0
    assert len(result.artifacts[0].bytes) > 0


def test_quill_render_with_explicit_format(taro_quill_dir, taro_md):
    """quill.render() honours an explicit OutputFormat argument."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))

    result = quill.render(taro_md, OutputFormat.SVG)

    assert len(result.artifacts) > 0
    assert result.output_format == OutputFormat.SVG


def test_quill_render_ref_mismatch_warning(taro_quill_dir):
    """Rendering a ParsedDocument with a mismatched QUILL ref emits a warning."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))

    # Build a document that names a different quill
    mismatch_md = (
        "---\n"
        "QUILL: completely_different_quill\n"
        "author: Test Author\n"
        "ice_cream: Chocolate\n"
        "title: Mismatch Test\n"
        "---\n\nContent.\n"
    )
    parsed = ParsedDocument.from_markdown(mismatch_md)
    result = quill.render(parsed)

    codes = [w.code for w in result.warnings]
    assert "quill::ref_mismatch" in codes, f"expected ref_mismatch warning, got: {codes}"
    assert len(result.artifacts) > 0, "artifact must still be produced"


def test_engine_workflow_still_works(taro_quill_dir, taro_md):
    """engine.workflow(quill) remains the correct path for dynamic-asset renders."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))
    workflow = engine.workflow(quill)

    parsed = ParsedDocument.from_markdown(taro_md)
    result = workflow.render(parsed, OutputFormat.PDF)

    assert len(result.artifacts) > 0
    assert result.output_format == OutputFormat.PDF
