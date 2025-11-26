use std::path::PathBuf;

use quillmark::{OutputFormat, ParsedDocument, Quillmark};
use quillmark_fixtures::write_example_output;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Markdown document without a QUILL tag - will use the default quill
    let markdown = r#"---
title: Default Quill Example
author: Quillmark Team
field1: "hello"
field2: "world"
---

# Introduction

This example demonstrates the **default Quill** functionality in Quillmark.

When a markdown document doesn't specify a `QUILL` tag in the frontmatter,
the system automatically uses the default Quill provided by the Typst backend.

## Features

The default Quill supports:

- **Bold text**
- _Italic text_
- `Code formatting`
- ~Strikethrough text~

## Benefits

1. No need to create a custom Quill for simple documents
2. Automatic rendering with sensible defaults
3. Quick prototyping and testing
"#;

    // Parse the markdown - note no QUILL tag, so default quill will be used
    let parsed = ParsedDocument::from_markdown(markdown)?;

    println!("QUILL tag in document: {:?}", parsed.quill_tag());
    println!("Will use default quill: __default__\n");

    // Create engine - default quill is automatically registered with Typst backend
    let engine = Quillmark::new();

    // Verify default quill is registered
    let registered_quills = engine.registered_quills();
    println!("Registered quills: {:?}\n", registered_quills);

    // Build workflow from parsed document - will use __default__ quill
    let workflow = engine
        .workflow(&parsed)
        .expect("Failed to create workflow");

    println!("Using quill: {}", workflow.quill_name());
    println!("Backend: {}", workflow.backend_id());
    println!("Supported formats: {:?}\n", workflow.supported_formats());

    // Process glue output
    let glue_output = workflow.process_glue(&parsed)?;
    write_example_output("default_quill_glue.json", glue_output.as_bytes())?;

    let output_dir = PathBuf::from("crates/fixtures/output/");

    println!(
        "Wrote glue output to: {}",
        output_dir.join("default_quill_glue.json").display()
    );

    // Render to PDF
    let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;
    if !result.artifacts.is_empty() {
        let pdf_bytes = &result.artifacts[0].bytes;
        write_example_output("default_quill.pdf", pdf_bytes)?;
        let pdf_path = output_dir.join("default_quill.pdf");
        println!("Wrote rendered PDF to: {}", pdf_path.display());
    } else {
        println!("No artifacts produced by render");
    }

    Ok(())
}
