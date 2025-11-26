"""Tests for Quillmark engine."""

import pytest

from quillmark import Quill, Quillmark


def test_engine_creation():
    """Test creating a Quillmark engine."""
    engine = Quillmark()
    assert "typst" in engine.registered_backends()
    assert len(engine.registered_quills()) == 1


def test_register_quill(taro_quill_dir):
    """Test registering a quill."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    assert quill.name in engine.registered_quills()


def test_workflow_from_quill_name(taro_quill_dir):
    """Test creating a workflow from quill name."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    workflow = engine.workflow(quill.name)
    assert workflow.quill_name == quill.name
    assert workflow.backend_id == quill.backend


def test_workflow_from_quill(taro_quill_dir):
    """Test creating a workflow from quill object."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    
    workflow = engine.workflow(quill)
    assert workflow.quill_name == quill.name
    assert workflow.backend_id == quill.backend
