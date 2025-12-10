"""Tests for Quill loading."""

import pytest

from quillmark import Quill, QuillmarkError


def test_load_quill(taro_quill_dir):
    """Test loading a quill from path."""
    quill = Quill.from_path(str(taro_quill_dir))
    assert quill.name == "taro"
    assert quill.backend == "typst"
    assert "Favorite Ice Cream" in quill.plate


def test_load_nonexistent_quill(tmp_path):
    """Test loading a non-existent quill."""
    with pytest.raises(QuillmarkError):
        Quill.from_path(str(tmp_path / "nonexistent"))


def test_quill_metadata(taro_quill_dir):
    """Test accessing quill metadata."""
    quill = Quill.from_path(str(taro_quill_dir))
    metadata = quill.metadata
    # Metadata is flattened from Quill.toml
    assert metadata.get("backend") == "typst"
    # Just verify we can access it
    assert isinstance(metadata, dict)
