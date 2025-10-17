"""Basic example of using quillmark."""

from pathlib import Path

from quillmark import OutputFormat, ParsedDocument, Quill, Quillmark

# Create engine
engine = Quillmark()

# Load and register quill (using taro fixture)
script_dir = Path(__file__).parent
repo_root = script_dir.parent.parent
quill_path = repo_root / "quillmark-fixtures" / "resources" / "taro"

if quill_path.exists():
    quill = Quill.from_path(str(quill_path))
    engine.register_quill(quill)

    # Parse markdown
    markdown = """---
title: Hello World
author: Alice
ice_cream: Chocolate
---

# Introduction

This is a **test** document about ice cream.
"""

    parsed = ParsedDocument.from_markdown(markdown)

    # Create workflow and render
    workflow = engine.workflow_from_quill_name(quill.name)
    result = workflow.render(parsed, OutputFormat.PDF)

    # Save output
    import tempfile
    output_path = Path(tempfile.gettempdir()) / "basic_example.pdf"
    result.artifacts[0].save(str(output_path))
    print(f"Generated {len(result.artifacts[0].bytes)} bytes to {output_path}")
else:
    print(f"Quill not found at {quill_path}")
    print("Please update the quill_path to point to a valid quill directory")
