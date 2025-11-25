"""Shared test fixtures for quillmark tests.

These fixtures prefer using the canonical repository fixtures located in
`crates/fixtures/resources`. If those resources cannot be found the
original simple fallbacks are used so tests remain robust in odd layouts.
"""

import shutil
from pathlib import Path
import pytest

WORKSPACE_ROOT = Path(__file__).resolve().parents[4]
RESOURCES_PATH = WORKSPACE_ROOT / "crates" / "fixtures" / "resources"
QUILLS_PATH = RESOURCES_PATH / "tonguetoquill-collection" / "quills"


@pytest.fixture
def taro_quill_dir():
    """Provide a test quill directory.

    This will copy an existing fixture from `quillmark-fixtures/resources`
    into the test temporary directory so tests can safely mutate files.
    The default fixture used is `appreciated_letter`.
    """
    fixture_path = QUILLS_PATH / "taro"

    assert fixture_path.exists(), f"Preferred fixture not found: {fixture_path}"

    return fixture_path


@pytest.fixture
def taro_md():
    """Return the example taro markdown."""
    sample_path = QUILLS_PATH / "taro" / "taro.md"

    if sample_path.exists():
        return sample_path.read_text()
    else:
        raise FileNotFoundError(f"Markdown example not found: {sample_path}")
