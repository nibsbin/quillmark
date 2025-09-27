use std::fs;
use quillmark_typst::{TypstBackend, Quill};
use quillmark_core::Backend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("QuillMark Typst Backend Example");
    
    // Create quill from the hello-quill example
    let quill_path = "examples/hello-quill";
    let quill = Quill::from_path(quill_path)?;
    let backend = TypstBackend::with_quill(quill_path)?;
    
    println!("Loaded quill: {}", quill.name);
    println!("Main file: {}", quill.main_path().display());
    println!("Packages: {}", quill.packages_path().display());
    println!("Assets: {}", quill.assets_path().display());
    
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
    
    match backend.render(markdown, &options) {
        Ok(artifacts) => {
            println!("✓ Successfully compiled {} artifact(s)", artifacts.len());
            
            for (i, artifact) in artifacts.iter().enumerate() {
                match artifact.output_format {
                    quillmark_core::OutputFormat::Pdf => {
                        let filename = format!("output{}.pdf", if i == 0 { "".to_string() } else { format!("_{}", i) });
                        fs::write(&filename, &artifact.bytes)?;
                        println!("  → Saved PDF: {} ({} bytes)", filename, artifact.bytes.len());
                    }
                    quillmark_core::OutputFormat::Svg => {
                        let filename = format!("output{}.svg", if i == 0 { "".to_string() } else { format!("_{}", i) });
                        fs::write(&filename, &artifact.bytes)?;
                        println!("  → Saved SVG: {} ({} bytes)", filename, artifact.bytes.len());
                    }
                    quillmark_core::OutputFormat::Txt => {
                        println!("  → Text format not supported by Typst backend");
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Compilation failed: {}", e);
            return Err(e.into());
        }
    }
    
    // Test SVG compilation
    println!("\nCompiling markdown to SVG...");
    let svg_options = quillmark_core::Options {
        backend: Some("typst".to_string()),
        format: Some(quillmark_core::OutputFormat::Svg),
    };
    
    match backend.render(markdown, &svg_options) {
        Ok(artifacts) => {
            println!("✓ Successfully compiled {} SVG page(s)", artifacts.len());
            
            for (i, artifact) in artifacts.iter().enumerate() {
                let filename = format!("output_page_{}.svg", i + 1);
                fs::write(&filename, &artifact.bytes)?;
                println!("  → Saved SVG page {}: {} ({} bytes)", i + 1, filename, artifact.bytes.len());
            }
        }
        Err(e) => {
            println!("✗ SVG compilation failed: {}", e);
        }
    }
    
    println!("\n🎉 Example completed successfully!");
    println!("Check the generated PDF and SVG files to see the results.");
    
    Ok(())
}