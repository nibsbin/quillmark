"""Tests for Quillmark engine."""

from quillmark import Quill, Quillmark


def test_workflow_from_quill(taro_quill_dir):
    """Test engine creates workflow from Quill object."""
    engine = Quillmark()
    quill = engine.quill_from_path(str(taro_quill_dir))

    workflow = engine.workflow(quill)
    assert quill.name in workflow.quill_ref
    assert workflow.backend_id == quill.backend
