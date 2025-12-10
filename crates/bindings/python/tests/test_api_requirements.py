"""Tests for the new API requirements from the major overhaul.

This test ensures that:
1. ParsedDocument has quill_tag property
2. Quill object includes: metadata, name, backend name, loaded example content, 
   field_schemas, and supported_formats
3. The workflow can be created and used as specified
"""

import pytest
from quillmark import Quillmark, Quill, ParsedDocument, OutputFormat
from conftest import QUILLS_PATH


def test_parsed_document_quill_tag():
    """Test that ParsedDocument exposes quill_tag property."""
    markdown_with_quill = """---
QUILL: my_quill
title: Test
---

# Content
"""
    parsed = ParsedDocument.from_markdown(markdown_with_quill)
    assert parsed.quill_tag() == "my_quill"
    
    markdown_without_quill = """---
title: Test
---

# Content
"""
    parsed2 = ParsedDocument.from_markdown(markdown_without_quill)
    assert parsed2.quill_tag() == "__default__"


def test_quill_properties(taro_quill_dir):
    """Test that Quill exposes all required properties."""
    quill = Quill.from_path(str(taro_quill_dir))
    
    # Verify all required properties are accessible
    assert quill.name == "taro"
    assert quill.backend == "typst"
    assert quill.plate is not None
    assert isinstance(quill.plate, str)
    assert len(quill.plate) > 0

    # Check metadata is accessible
    metadata = quill.metadata
    assert isinstance(metadata, dict)
    assert metadata.get("backend") == "typst"

    # Check schema is accessible
    schema = quill.schema
    assert isinstance(schema, dict)

    # Check example content
    example = quill.example
    assert example is not None
    assert isinstance(example, str)
    assert len(example) > 0
    
    # Check supported_formats (new requirement)
    supported_formats = quill.supported_formats()
    assert isinstance(supported_formats, list)
    assert len(supported_formats) > 0
    assert OutputFormat.PDF in supported_formats


def test_workflow_from_parsed_with_quill_tag(taro_quill_dir, taro_md):
    """Test workflow_from_parsed works with QUILL tag."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    # Add QUILL tag to markdown
    markdown_with_tag = f"""
{taro_md}
"""
    
    parsed = ParsedDocument.from_markdown(markdown_with_tag)
    assert parsed.quill_tag() == quill.name
    
    # Create workflow from parsed document
    workflow = engine.workflow(parsed)
    assert workflow.quill_name == quill.name
    assert workflow.backend_id == quill.backend


def test_full_workflow_as_specified():
    """Test the full workflow as described in the problem statement.
    
    1. Load Quill
    2. Parse markdown into ParsedDocument
    3. Retrieve Quill object with all properties
    4. Render with options
    5. Get RenderResult with artifacts
    """
    # Step 1: Create engine and load quill
    engine = Quillmark()
    
    # Find taro quill
    from pathlib import Path
    repo_root = Path(__file__).resolve().parents[2]
    taro_dir = QUILLS_PATH / "taro"
    
    quill = Quill.from_path(str(taro_dir))
    engine.register_quill(quill)
    
    # Step 2: Parse markdown (with quill tag)
    markdown = """---
QUILL: taro
author: Test Author
ice_cream: Chocolate
title: Test Document
---

# My Document

This is a test.
"""
    parsed = ParsedDocument.from_markdown(markdown)
    
    # Verify ParsedDocument.quill_tag
    assert parsed.quill_tag() == "taro"
    
    # Step 3: Retrieve Quill object and inspect properties
    # Consumer can use this information to configure render options
    assert quill.name == "taro"
    assert quill.backend == "typst"
    assert quill.example is not None
    assert quill.schema is not None
    assert quill.metadata is not None
    
    # Get supported formats from quill
    supported_formats = quill.supported_formats()
    assert OutputFormat.PDF in supported_formats
    
    # Step 4: Create workflow (can be inferred from ParsedDocument or explicit)
    # Option 1: Infer from ParsedDocument.quill_tag
    workflow = engine.workflow(parsed)
    
    # Option 2: Explicit by name
    # workflow = engine.workflow("taro")
    
    # Option 3: Explicit by Quill object
    # workflow = engine.workflow(quill)
    
    # Verify workflow properties
    assert workflow.quill_name == "taro"
    assert workflow.backend_id == "typst"
    assert OutputFormat.PDF in workflow.supported_formats
    
    # Step 5: Render with options
    result = workflow.render(parsed, OutputFormat.PDF)
    
    # Verify RenderResult
    assert len(result.artifacts) > 0
    assert result.artifacts[0].output_format == OutputFormat.PDF
    assert len(result.artifacts[0].bytes) > 0
    
    # Check for warnings (non-errors)
    assert isinstance(result.warnings, list)


def test_render_without_quill_tag(taro_quill_dir, taro_md):
    """Test that render works without QUILL tag when quill is specified explicitly."""
    engine = Quillmark()
    quill = Quill.from_path(str(taro_quill_dir))
    engine.register_quill(quill)
    
    # Parse markdown without QUILL tag
    parsed = ParsedDocument.from_markdown(taro_md)
    
    # Create workflow explicitly by name
    workflow = engine.workflow(quill.name)
    
    # Render should work
    result = workflow.render(parsed, OutputFormat.PDF)
    assert len(result.artifacts) > 0
