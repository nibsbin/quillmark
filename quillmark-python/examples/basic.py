"""Basic example of using quillmark."""

from pathlib import Path

from quillmark import OutputFormat, ParsedDocument, Quill, Quillmark

# Create engine
engine = Quillmark()

# Load and register quill from fixtures
# This example uses the taro template from quillmark-fixtures
quill_path = Path(__file__).parent.parent.parent / "quillmark-fixtures" / "resources" / "taro"
if quill_path.exists():
    quill = Quill.from_path(str(quill_path))
    engine.register_quill(quill)

    # Parse markdown compatible with taro template
    markdown = """---
author: Alice
ice_cream: chocolate
title: Hello World
---

# Introduction

This is a **test** document.
"""

    parsed = ParsedDocument.from_markdown(markdown)

    # Create workflow and render
    workflow = engine.workflow_from_quill_name(quill.name)
    result = workflow.render(parsed, OutputFormat.PDF)

    # Save output
    output_path = Path("output.pdf")
    result.artifacts[0].save(str(output_path))
    print(f"Generated {len(result.artifacts[0].bytes)} bytes to {output_path}")
else:
    print(f"Quill not found at {quill_path}")
    print("Please update the quill_path to point to a valid quill directory")
