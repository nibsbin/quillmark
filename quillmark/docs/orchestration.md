# Orchestration

Orchestrates the Quillmark engine and its workflows.

---

# Quillmark Engine

High-level engine for orchestrating backends and quills.

`Quillmark` manages the registration of backends and quills, and provides
a convenient way to create workflows. Backends are automatically registered
based on enabled crate features.

## Backend Auto-Registration

When a `Quillmark` engine is created with [`Quillmark::new`], it automatically
registers all backends based on enabled features:

- **typst** (default) - Typst backend for PDF/SVG rendering

## Workflow (Engine Level)

1. Create an engine with [`Quillmark::new`]
2. Register quills with [`crate::orchestration::Quillmark::register_quill()`]
3. Load workflows with [`crate::orchestration::Quillmark::load()`]
4. Render documents using the workflow

## Examples

### Basic Usage

```no_run
use quillmark::{Quillmark, Quill, OutputFormat};

// Step 1: Create engine with auto-registered backends
let mut engine = Quillmark::new();

// Step 2: Create and register quills
let quill = Quill::from_path("path/to/quill").unwrap();
engine.register_quill(quill);

// Step 3: Load workflow by quill name
let workflow = engine.load("my-quill").unwrap();

// Step 4: Render markdown
let result = workflow.render("# Hello", Some(OutputFormat::Pdf)).unwrap();
```

### Loading by Reference

```no_run
# use quillmark::{Quillmark, Quill};
# let mut engine = Quillmark::new();
let quill = Quill::from_path("path/to/quill").unwrap();
engine.register_quill(quill.clone());

// Load by name
let workflow1 = engine.load("my-quill").unwrap();

// Load by object (doesn't need to be registered)
let workflow2 = engine.load(&quill).unwrap();
```

### Inspecting Engine State

```no_run
# use quillmark::Quillmark;
# let engine = Quillmark::new();
println!("Available backends: {:?}", engine.registered_backends());
println!("Registered quills: {:?}", engine.registered_quills());
```

---

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
