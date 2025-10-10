"""Shared test fixtures for quillmark tests."""

from pathlib import Path

import pytest


def _get_fixtures_path():
    """Get the path to quillmark-fixtures/resources."""
    # Navigate from quillmark-python/tests to workspace root, then to fixtures
    test_dir = Path(__file__).parent
    python_dir = test_dir.parent
    workspace_root = python_dir.parent
    fixtures_path = workspace_root / "quillmark-fixtures" / "resources"
    return fixtures_path


@pytest.fixture
def fixtures_path():
    """Provide path to quillmark-fixtures/resources."""
    return _get_fixtures_path()


@pytest.fixture
def test_quill_dir(fixtures_path):
    """Return path to the taro quill template from fixtures."""
    return fixtures_path / "taro"


@pytest.fixture
def simple_markdown(fixtures_path):
    """Return simple test markdown compatible with taro template."""
    return """---
author: Test Author
ice_cream: vanilla
title: Test Document
---

# Sample Document

This is a **bold** statement with *emphasis* and some `inline code`.
"""
