use std::fs;
use quillmark_typst::TypstBackend;
use quillmark_core::test_context;
use quillmark::{register_backend, render, Options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("QuillMark Typst Backend Example");
    
    // Use the test context helper to find the examples directory
    let examples_dir = test_context::examples_dir().map_err(|e| -> Box<dyn std::error::Error> { e })?;
    println!("Examples directory: {}", examples_dir.display());
    
    // Register the Typst backend
    register_backend(Box::new(TypstBackend::new()));
    
    // Create output directory within examples
    let output_dir = test_context::create_output_dir("output").map_err(|e| -> Box<dyn std::error::Error> { e })?;
    
    // Sample markdown content
    let markdown = r#"# Welcome to QuillMark

This is a **sample document** written in markdown that will be compiled using the Typst backend.

## Features

- Markdown to Typst compilation
- Dynamic template loading  
- PDF and SVG output support

## Code Example

```rust
let backend = TypstBackend::new();
register_backend(Box::new(backend));
```

> This demonstrates how easy it is to create beautiful documents with QuillMark!"#;

    println!("\nCompiling markdown to PDF...");
    
    // Get path to quill template
    let quill_path = examples_dir.join("hello-quill");
    
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