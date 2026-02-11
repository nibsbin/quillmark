"""Tests for versioned quill retrieval."""
import pytest
from quillmark import Quillmark, Quill

def test_get_quill_versioning(taro_quill_dir):
    """Test retrieving quills with version selectors."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    
    # Register the quill (assuming it has a version in Quill.yaml, e.g., 1.0)
    engine.register_quill(quill)
    
    # Test valid retrieval by name (implicitly latest)
    retrieved = engine.get_quill("taro")
    assert retrieved is not None
    assert retrieved.name == "taro"
    
    # Test valid retrieval by exact version (assuming taro is 1.0 or similar)
    # We first check the version to be sure
    version = quill.metadata.get("version")
    assert version is not None
    
    retrieved_version = engine.get_quill(f"taro@{version}")
    assert retrieved_version is not None
    assert retrieved_version.name == "taro"
    
    # Test retrieval by major version
    major = version.split('.')[0]
    retrieved_major = engine.get_quill(f"taro@{major}")
    assert retrieved_major is not None
    assert retrieved_major.name == "taro"
    
    # Test invalid version
    retrieved_invalid = engine.get_quill("taro@99.99")
    assert retrieved_invalid is None
    
    # Test invalid name
    retrieved_bad_name = engine.get_quill("nonexistent")
    assert retrieved_bad_name is None
