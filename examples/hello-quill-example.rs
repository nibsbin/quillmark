use std::fs;
use quillmark_typst::{TypstBackend, Quill};
use quillmark_core::{Backend, test_context};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("QuillMark Typst Backend Example");
    
    // Use the test context helper to find the examples directory
    let examples_dir = test_context::examples_dir().map_err(|e| -> Box<dyn std::error::Error> { e })?;
    println!("Examples directory: {}", examples_dir.display());
    
    // Create quill from the hello-quill example
    let quill_path = examples_dir.join("hello-quill");
    let quill = Quill::from_path(&quill_path)?;
    let backend = TypstBackend::with_quill(&quill_path)?;
    
    println!("Loaded quill: {}", quill.name);
    println!("Main file: {}", quill.main_path().display());
    println!("Packages: {}", quill.packages_path().display());
    println!("Assets: {}", quill.assets_path().display());
    
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
let backend = TypstBackend::with_quill("hello-quill")?;
```

> This demonstrates how easy it is to create beautiful documents with QuillMark!"#;

    println!("\nCompiling markdown to PDF...");
    
    // Test compilation
    let options = quillmark_core::Options {
        backend: Some("typst".to_string()),
        format: Some(quillmark_core::OutputFormat::Pdf),
    };
    
    let artifacts = backend.render(markdown, &options).unwrap();
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
    let svg_options = quillmark_core::Options {
        backend: Some("typst".to_string()),
        format: Some(quillmark_core::OutputFormat::Svg),
    };
    
    // Panic on SVG render errors as well so we can inspect a backtrace
    let artifacts = backend.render(markdown, &svg_options).unwrap();
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