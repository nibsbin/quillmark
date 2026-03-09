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
QUILLS_PATH = RESOURCES_PATH / "quills"


def _latest_version(quill_dir: Path) -> Path:
    """Return the latest versioned subdirectory of a quill, or the dir itself."""
    if (quill_dir / "Quill.yaml").exists():
        return quill_dir
    versions = sorted(p.name for p in quill_dir.iterdir() if p.is_dir())
    if versions:
        return quill_dir / versions[-1]
    return quill_dir


@pytest.fixture
def taro_quill_dir():
    """Provide a test quill directory.

    This will copy an existing fixture from `quillmark-fixtures/resources`
    into the test temporary directory so tests can safely mutate files.
    The default fixture used is `taro`.
    """
    fixture_path = _latest_version(QUILLS_PATH / "taro")

    assert fixture_path.exists(), f"Preferred fixture not found: {fixture_path}"

    return fixture_path


@pytest.fixture
def taro_md():
    """Return the example taro markdown."""
    sample_path = _latest_version(QUILLS_PATH / "taro") / "example.md"

    if sample_path.exists():
        return sample_path.read_text()
    else:
        raise FileNotFoundError(f"Markdown example not found: {sample_path}")
