#!/usr/bin/env python3
"""Example demonstrating the new quillmark-python API workflow.

This example shows the opinionated visibility over the rendering workflow:
1. Load Quill
2. Parse markdown into ParseDocument
3. Retrieve Quill object with metadata and properties
4. Configure render options based on Quill properties
5. Render and get RenderResult
"""

from pathlib import Path
from quillmark import Quillmark, Quill, ParsedDocument, OutputFormat


def main():
    # Find the taro quill in fixtures
    script_dir = Path(__file__).parent
    repo_root = script_dir.parent.parent.parent.parent
    taro_dir = repo_root / "crates" / "fixtures" / "resources" / "tonguetoquill-collection" / "quills" / "taro"
    
    if not taro_dir.exists():
        print(f"Error: Could not find taro quill at {taro_dir}")
        return
    
    print("=== Quillmark Python API Workflow Demo ===\n")
    
    # Step 1: Create engine and load quill
    print("Step 1: Loading Quill...")
    engine = Quillmark()
    quill = Quill.from_path(str(taro_dir))
    engine.register_quill(quill)
    print(f"  ✓ Loaded quill: {quill.name}")
    print(f"  ✓ Registered backends: {engine.registered_backends()}")
    print()
    
    # Step 2: Parse markdown into ParsedDocument
    print("Step 2: Parsing Markdown...")
    markdown = """---
QUILL: taro
author: Alice
ice_cream: Taro
title: My Favorite Ice Cream
---

# Introduction

I love **Taro** ice cream! It has a unique flavor that's both:
- Nutty
- Earthy
- Sweet

## Why Taro?

Taro ice cream originated in Asia and has become popular worldwide.
The purple color is distinctive and the taste is unforgettable.

---
SCOPE: quotes
author: Mark Twain
---
The secret of getting ahead is getting started... with taro ice cream.
"""
    
    parsed = ParsedDocument.from_markdown(markdown)
    print(f"  ✓ Parsed document")
    print(f"  ✓ Quill tag from document: {parsed.quill_tag()}")
    print(f"  ✓ Document fields: {list(parsed.fields.keys())}")
    print()
    
    # Step 3: Retrieve Quill object and inspect properties
    print("Step 3: Inspecting Quill Properties...")
    print(f"  • Name: {quill.name}")
    print(f"  • Backend: {quill.backend}")
    print(f"  • Has example content: {quill.example is not None}")
    print(f"  • Field schemas: {quill.schema}")
    
    
    # Step 4: Create workflow and configure render options
    print("Step 4: Creating Workflow...")
    
    # Option 1: Infer from ParsedDocument.quill_tag (recommended)
    workflow = engine.workflow(parsed)
    
    # Alternative options:
    # workflow = engine.workflow("taro")
    # workflow = engine.workflow(quill)

    # Get supported formats - consumer can use this to configure render options
    supported_formats = workflow.supported_formats
    print(f"  • Supported formats: {supported_formats}")
    print()
    
    print(f"  ✓ Created workflow for quill: {workflow.quill_name}")
    print(f"  ✓ Backend: {workflow.backend_id}")
    print(f"  ✓ Workflow supported formats: {workflow.supported_formats}")
    print()
    
    # Step 5: Render with options
    print("Step 5: Rendering...")
    
    # Choose format from supported formats
    if OutputFormat.PDF in supported_formats:
        render_format = OutputFormat.PDF
    else:
        render_format = supported_formats[0]
    
    print(f"  • Rendering to {render_format}...")
    result = workflow.render(parsed, render_format)
    
    # Step 6: Process RenderResult
    print(f"  ✓ Generated {len(result.artifacts)} artifact(s)")
    
    for i, artifact in enumerate(result.artifacts):
        print(f"  • Artifact {i + 1}:")
        print(f"    - Format: {artifact.output_format}")
        print(f"    - Size: {len(artifact.bytes):,} bytes")
        
        # Save artifact
        output_name = "pdf" if artifact.output_format == OutputFormat.PDF else "svg" if artifact.output_format == OutputFormat.SVG else "txt"
        output_path = Path(f"/tmp/taro_example.{output_name}")
        artifact.save(str(output_path))
        print(f"    - Saved to: {output_path}")
    
    # Check for warnings
    if result.warnings:
        print(f"\n  ⚠ Warnings ({len(result.warnings)}):")
        for warning in result.warnings:
            print(f"    - {warning.severity}: {warning.message}")
    else:
        print(f"\n  ✓ No warnings")
    
    print("\n=== Demo Complete ===")


if __name__ == "__main__":
    main()
