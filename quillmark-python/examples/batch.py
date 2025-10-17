"""Example of batch processing markdown files."""

from pathlib import Path
import tempfile

from quillmark import OutputFormat, ParsedDocument, Quillmark, Quill

# Create engine
engine = Quillmark()

# Load taro quill from fixtures
script_dir = Path(__file__).parent
repo_root = script_dir.parent.parent
quill_path = repo_root / "quillmark-fixtures" / "resources" / "taro"

if quill_path.exists():
    quill = Quill.from_path(str(quill_path))
    engine.register_quill(quill)
    
    # Create workflow
    workflow = engine.workflow_from_quill_name(quill.name)
    
    # Create temporary directory with sample markdown files
    with tempfile.TemporaryDirectory() as tmpdir:
        markdown_dir = Path(tmpdir) / "documents"
        output_dir = Path(tmpdir) / "output"
        markdown_dir.mkdir()
        output_dir.mkdir()
        
        # Create sample markdown files
        sample_docs = [
            ("doc1.md", """---
title: First Document
author: Alice
ice_cream: Vanilla
---

# Document 1

This is the first document."""),
            ("doc2.md", """---
title: Second Document
author: Bob
ice_cream: Chocolate
---

# Document 2

This is the second document."""),
            ("doc3.md", """---
title: Third Document
author: Carol
ice_cream: Strawberry
---

# Document 3

This is the third document."""),
        ]
        
        for filename, content in sample_docs:
            (markdown_dir / filename).write_text(content)
        
        # Process multiple markdown files
        for md_file in sorted(markdown_dir.glob("*.md")):
            print(f"Processing {md_file.name}...")
            
            # Read and parse markdown
            content = md_file.read_text()
            parsed = ParsedDocument.from_markdown(content)
            
            # Render to PDF
            result = workflow.render(parsed, OutputFormat.PDF)
            
            # Save output
            output_path = output_dir / md_file.with_suffix('.pdf').name
            result.artifacts[0].save(str(output_path))
            
            print(f"  -> {output_path} ({len(result.artifacts[0].bytes):,} bytes)")
        
        print(f"\nProcessed {len(list(markdown_dir.glob('*.md')))} files")
        print(f"Output saved to: {output_dir}")
else:
    print(f"Quill not found at {quill_path}")
    print("Please update the paths to valid directories")
