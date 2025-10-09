"""Example of batch processing markdown files."""

from pathlib import Path

from quillmark import OutputFormat, ParsedDocument, Quillmark

# Create engine
engine = Quillmark()

# Example quill path
quill_path = Path("path/to/quill")

if quill_path.exists():
    from quillmark import Quill
    
    quill = Quill.from_path(str(quill_path))
    engine.register_quill(quill)
    
    # Create workflow
    workflow = engine.workflow_from_quill_name(quill.name)
    
    # Process multiple markdown files
    markdown_dir = Path("documents")
    output_dir = Path("output")
    output_dir.mkdir(exist_ok=True)
    
    if markdown_dir.exists():
        for md_file in markdown_dir.glob("*.md"):
            print(f"Processing {md_file.name}...")
            
            # Read and parse markdown
            content = md_file.read_text()
            parsed = ParsedDocument.from_markdown(content)
            
            # Render to PDF
            result = workflow.render(parsed, OutputFormat.PDF)
            
            # Save output
            output_path = output_dir / md_file.with_suffix('.pdf').name
            result.artifacts[0].save(str(output_path))
            
            print(f"  -> {output_path}")
        
        print(f"\nProcessed {len(list(markdown_dir.glob('*.md')))} files")
    else:
        print(f"Markdown directory not found: {markdown_dir}")
else:
    print(f"Quill not found at {quill_path}")
    print("Please update the paths to valid directories")
