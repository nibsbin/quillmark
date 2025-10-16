# quillmark-acroform

AcroForm backend for Quillmark that fills PDF form fields with templated values.

## Overview

This crate provides an AcroForm backend implementation that fills PDF form fields
with values rendered from YAML context using MiniJinja templates.

## Usage

The AcroForm backend is automatically registered when you create a Quillmark engine
with the `acroform` feature enabled.

```rust
use quillmark::{Quillmark, Quill, ParsedDocument, OutputFormat};

// Create engine with acroform backend
let mut engine = Quillmark::new();

// Load a quill with backend = "acroform"
let quill = Quill::from_path("path/to/acroform_quill").unwrap();
engine.register_quill(quill);

// Create workflow
let workflow = engine.workflow_from_quill_name("my_form").unwrap();

// Parse markdown with frontmatter
let markdown = r#"---
firstName: "John"
lastName: "Doe"
---
"#;
let parsed = ParsedDocument::from_markdown(markdown).unwrap();

// Render to PDF
let result = workflow.render(&parsed, Some(OutputFormat::Pdf)).unwrap();
```

## Quill Structure

An AcroForm quill must have the following structure:

```
my_form_quill/
├── Quill.toml
├── form.pdf
└── example.md
```

The `Quill.toml` must specify `backend = "acroform"`:

```toml
[Quill]
name = "my_form"
backend = "acroform"
example = "example.md"
description = "My PDF form"
```

## How It Works

1. Reads the PDF form from the quill's `form.pdf` file
2. Extracts field names and current values
3. For each field containing a template expression (e.g., `{{ firstName }}`), renders it with the JSON context
4. Writes the rendered values back to the PDF
5. Returns the filled PDF as bytes

## License

Apache-2.0
