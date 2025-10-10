"""Shared test fixtures for quillmark tests.

These fixtures prefer using the canonical repository fixtures located in
`quillmark-fixtures/resources`. If those resources cannot be found the
original simple fallbacks are used so tests remain robust in odd layouts.
"""

import shutil
from pathlib import Path
import pytest


@pytest.fixture
def taro_quill_dir():
    """Provide a test quill directory.

    This will copy an existing fixture from `quillmark-fixtures/resources`
    into the test temporary directory so tests can safely mutate files.
    The default fixture used is `appreciated_letter`.
    """
    repo_root = Path(__file__).resolve().parents[2]
    resources_dir = repo_root / "quillmark-fixtures" / "resources"
    fixture_path = resources_dir / "taro"

    assert fixture_path.exists(), f"Preferred fixture not found: {fixture_path}"


    return fixture_path


@pytest.fixture
def taro_md():
    """Return simple test markdown.

    Prefer the repository `sample.md` fixture when available.
    """
    repo_root = Path(__file__).resolve().parents[2]
    sample_path = repo_root / "quillmark-fixtures" / "resources" / "taro" / "taro.md"

    if sample_path.exists():
        return sample_path.read_text()

    return """---
title: Test Document
---

# Hello World

This is a test document.
"""
