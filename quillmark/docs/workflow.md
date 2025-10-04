# Workflow

Sealed workflow for rendering Markdown documents.

`Workflow` encapsulates the complete rendering pipeline from Markdown to final artifacts.
It manages the backend, quill template, and dynamic assets, providing methods for
rendering at different stages of the pipeline.

## Rendering Pipeline

The workflow supports rendering at three levels:

1. **Full render** ([`crate::orchestration::Workflow::render()`]) - Parse Markdown → Compose with template → Compile to artifacts
2. **Content render** ([`crate::orchestration::Workflow::render_content()`]) - Skip parsing, render pre-composed content
3. **Glue only** ([`crate::orchestration::Workflow::process_glue()`]) - Parse and compose, return template output

## Examples

### Basic Rendering

```no_run
# use quillmark::{Quillmark, OutputFormat};
# let mut engine = Quillmark::new();
# let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
# engine.register_quill(quill);
let workflow = engine.load("my-quill").unwrap();

let markdown = r#"---
title: "My Document"
author: "Alice"
---

# Introduction

This is my document.
"#;

let result = workflow.render(markdown, Some(OutputFormat::Pdf)).unwrap();
```

### Dynamic Assets (Builder Pattern)

```no_run
# use quillmark::{Quillmark, OutputFormat};
# let mut engine = Quillmark::new();
# let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
# engine.register_quill(quill);
let workflow = engine.load("my-quill").unwrap()
    .with_asset("logo.png", vec![/* PNG bytes */]).unwrap()
    .with_asset("chart.svg", vec![/* SVG bytes */]).unwrap();

let result = workflow.render("# Report", Some(OutputFormat::Pdf)).unwrap();
```

### Inspecting Workflow Properties

```no_run
# use quillmark::Quillmark;
# let mut engine = Quillmark::new();
# let quill = quillmark::Quill::from_path("path/to/quill").unwrap();
# engine.register_quill(quill);
let workflow = engine.load("my-quill").unwrap();

println!("Backend: {}", workflow.backend_id());
println!("Quill: {}", workflow.quill_name());
println!("Formats: {:?}", workflow.supported_formats());
```
