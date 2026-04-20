"""Tests for Quill loading via the engine."""

import pytest

from quillmark import Quillmark, QuillmarkError


def test_load_nonexistent_quill(tmp_path):
    """engine.quill_from_path raises on a missing directory."""
    engine = Quillmark()
    with pytest.raises(QuillmarkError):
        engine.quill_from_path(str(tmp_path / "nonexistent"))
