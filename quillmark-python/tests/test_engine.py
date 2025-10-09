"""Tests for Quillmark engine."""

import pytest

from quillmark import Quill, Quillmark


def test_engine_creation():
    """Test creating a Quillmark engine."""
    engine = Quillmark()
    assert "typst" in engine.registered_backends()
    assert len(engine.registered_quills()) == 0


def test_register_quill(test_quill_dir):
    """Test registering a quill."""
    engine = Quillmark()
    quill = Quill.from_path(str(test_quill_dir))
    engine.register_quill(quill)
    assert "test-quill" in engine.registered_quills()


def test_workflow_from_quill_name(test_quill_dir):
    """Test creating a workflow from quill name."""
    engine = Quillmark()
    quill = Quill.from_path(str(test_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow_from_quill_name("test-quill")
    assert workflow.quill_name() == "test-quill"
    assert workflow.backend_id() == "typst"


def test_workflow_from_quill(test_quill_dir):
    """Test creating a workflow from quill object."""
    engine = Quillmark()
    quill = Quill.from_path(str(test_quill_dir))
    
    workflow = engine.workflow_from_quill(quill)
    assert workflow.quill_name() == "test-quill"
    assert workflow.backend_id() == "typst"
