# Typst Compilation

This module compiles Typst documents to output formats (PDF and SVG).

## Functions

- [`compile::compile_to_pdf()`] - Compile Typst to PDF format
- [`compile::compile_to_svg()`] - Compile Typst to SVG format (one file per page)

## Quick Example

```rust,no_run
use quillmark_typst::compile::compile_to_pdf;
use quillmark_core::Quill;

let quill = Quill::from_path("path/to/quill")?;
let typst_content = "#set document(title: \"Test\")\n= Hello";

let pdf_bytes = compile_to_pdf(&quill, typst_content)?;
std::fs::write("output.pdf", pdf_bytes)?;
# Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
```

## Process

1. Creates a `QuillWorld` with the quill's assets and packages
2. Compiles the Typst document using the Typst compiler
3. Converts to target format (PDF or SVG)
4. Returns output bytes

## Detailed Documentation

See **[API.md](API.md)** for complete compilation API documentation.
