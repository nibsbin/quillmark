use std::fs;
use quillmark_typst::TypstBackend;
use quillmark_core::test_context;
use quillmark::{register_backend, render, Options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("QuillMark Typst Backend Example");
    
    // Use the test context helper to find the examples directory
    let examples_dir = test_context::examples_dir().map_err(|e| -> Box<dyn std::error::Error> { e })?;
    println!("Examples directory: {}", examples_dir.display());
    
    // Check if hello-quill exists, if not look at workspace level
    let quill_path = if examples_dir.join("hello-quill").exists() {
        examples_dir.join("hello-quill")
    } else {
        // Try workspace level - go up two levels from quillmark/examples to workspace root, then examples
        examples_dir.parent()
            .ok_or("Could not find parent directory")?
            .parent()
            .ok_or("Could not find grandparent directory")?
            .join("examples")
            .join("hello-quill")
    };
    
    println!("Looking for quill at: {}", quill_path.display());
    if !quill_path.exists() {
        return Err(format!("Quill path does not exist: {}", quill_path.display()).into());
    }
    
    // Register the Typst backend
    register_backend(Box::new(TypstBackend::new()));
    
    // Create output directory within examples
    let output_dir = test_context::create_output_dir("output").map_err(|e| -> Box<dyn std::error::Error> { e })?;
    
    // Sample markdown content with complete frontmatter
    let markdown = r#"---
letterhead_title: "DEPARTMENT OF THE AIR FORCE"
letterhead_caption:
  - "HEADQUARTERS UNITED STATES AIR FORCE"  
  - "WASHINGTON, DC 20330-1000"
date: "1 January 2024"
memo_for:
  - "ALL PERSONNEL"
memo_from:
  - "COMMANDER"
subject: "Test Subject from QuillMark"
references: []
cc: []
distribution: []
attachments: []
signature_block:
  - "JOHN DOE, Colonel, USAF"
  - "Commander"
---

# Welcome to QuillMark

This is a **sample document** written in markdown that will be compiled using the Typst backend.

## Features

- Markdown to Typst compilation
- Dynamic template loading  
- PDF and SVG output support
- Backend registration system

## Code Example

```rust
let backend = TypstBackend::new();
register_backend(Box::new(backend));
```

> This demonstrates how easy it is to create beautiful documents with QuillMark!"#;

    println!("\nCompiling markdown to PDF...");
    
    // Get path to quill template - should be at workspace level
    
    // Test compilation
    let options = Options {
        backend: Some("typst".to_string()),
        format: Some(quillmark_core::OutputFormat::Pdf),
        quill_path: Some(quill_path.clone()),
    };
    
    let artifacts = render(markdown, &options).unwrap();
    println!("✓ Successfully compiled {} artifact(s)", artifacts.len());

    for (i, artifact) in artifacts.iter().enumerate() {
        match artifact.output_format {
            quillmark_core::OutputFormat::Pdf => {
                let filename = if i == 0 { "output.pdf".to_string() } else { format!("output_{}.pdf", i) };
                let filepath = output_dir.join(&filename);
                fs::write(&filepath, &artifact.bytes)?;
                println!("  → Saved PDF: {} ({} bytes)", filepath.display(), artifact.bytes.len());
            }
            quillmark_core::OutputFormat::Svg => {
                let filename = if i == 0 { "output.svg".to_string() } else { format!("output_{}.svg", i) };
                let filepath = output_dir.join(&filename);
                fs::write(&filepath, &artifact.bytes)?;
                println!("  → Saved SVG: {} ({} bytes)", filepath.display(), artifact.bytes.len());
            }
            quillmark_core::OutputFormat::Txt => {
                println!("  → Text format not supported by Typst backend");
            }
        }
    }
    
    // Test SVG compilation
    println!("\nCompiling markdown to SVG...");
    let svg_options = Options {
        backend: Some("typst".to_string()),
        format: Some(quillmark_core::OutputFormat::Svg),
        quill_path: Some(quill_path),
    };
    
    // Panic on SVG render errors as well so we can inspect a backtrace
    let artifacts = render(markdown, &svg_options).unwrap();
    println!("✓ Successfully compiled {} SVG page(s)", artifacts.len());

    for (i, artifact) in artifacts.iter().enumerate() {
        let filename = format!("output_page_{}.svg", i + 1);
        let filepath = output_dir.join(&filename);
        fs::write(&filepath, &artifact.bytes)?;
        println!("  → Saved SVG page {}: {} ({} bytes)", i + 1, filepath.display(), artifact.bytes.len());
    }
    
    println!("\n🎉 Example completed successfully!");
    println!("Check the generated files in: {}", output_dir.display());
    
    Ok(())
}