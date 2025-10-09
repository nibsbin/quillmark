"""Shared test fixtures for quillmark tests."""

import tempfile
from pathlib import Path

import pytest


@pytest.fixture
def test_quill_dir(tmp_path):
    """Create a test quill directory."""
    quill_path = tmp_path / "test-quill"
    quill_path.mkdir()
    
    # Create Quill.toml
    (quill_path / "Quill.toml").write_text(
        """[Quill]
name = "test-quill"
backend = "typst"
glue = "glue.typ"
"""
    )
    
    # Create glue template - use simpler template without filters
    (quill_path / "glue.typ").write_text(
        """#set page(width: 100pt, height: 100pt)

#text(size: 16pt, weight: "bold")[Test Document]

Hello World

This is a test document.
"""
    )
    
    return quill_path


@pytest.fixture
def simple_markdown():
    """Return simple test markdown."""
    return """---
title: Test Document
---

# Hello World

This is a test document.
"""
