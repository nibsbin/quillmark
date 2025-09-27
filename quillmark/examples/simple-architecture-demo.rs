use quillmark_typst::TypstBackend;
use quillmark::{register_backend, render, Options};
use quillmark_core::{OutputFormat, test_context};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple QuillMark Example");
    
    // Register the Typst backend
    register_backend(Box::new(TypstBackend::new()));
    
    // Find workspace examples directory 
    let examples_dir = test_context::examples_dir()
        .map_err(|e| -> Box<dyn std::error::Error> { e })?;
    let quill_path = examples_dir.parent()
        .ok_or("Could not find parent directory")?
        .parent()
        .ok_or("Could not find grandparent directory")?
        .join("examples")
        .join("simple-quill");
    
    println!("Using quill at: {}", quill_path.display());
    
    // Simple markdown with frontmatter
    let markdown = r#"---
title: "QuillMark Architecture Demo"
---

# New Architecture Working!

The QuillMark library has been successfully restructured:

## Backend Trait Changes

The **Backend** trait now provides only modular behaviors:
- `id()` - backend identifier
- `supported_formats()` - supported output formats  
- `glue_type()` - file extension (e.g., ".typ")
- `register_filters()` - register Tera filters
- `compile()` - compile rendered content

## Orchestration Logic

All orchestration logic has been moved to the `quillmark` crate:
- Backend selection
- Quill template loading
- Markdown parsing and frontmatter extraction
- Template rendering with filters
- Backend compilation coordination

## Benefits

- **Separation of concerns**: Orchestration vs backend-specific logic
- **Modular design**: Backends focus only on their compilation logic
- **Extensibility**: Easy to add new backends
- **Testability**: Clear interfaces between components

This demonstrates the successful implementation of the requested architecture changes!"#;

    let options = Options {
        backend: Some("typst".to_string()),
        format: Some(OutputFormat::Pdf),
        quill_path: Some(quill_path),
    };
    
    println!("Rendering with new architecture...");
    let artifacts = render(markdown, &options)?;
    
    println!("✓ Successfully compiled {} artifact(s)", artifacts.len());
    println!("  PDF size: {} bytes", artifacts[0].bytes.len());
    
    // Save the output to the examples directory
    let output_dir = examples_dir.join("output");
    fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join("architecture_demo.pdf");
    fs::write(&output_path, &artifacts[0].bytes)?;
    println!("  Saved to: {}", output_path.display());
    
    println!("\n🎉 Architecture restructuring successful!");
    
    Ok(())
}