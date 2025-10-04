# Quillmark Engine

High-level engine for orchestrating backends and quills.

`Quillmark` manages the registration of backends and quills, and provides
a convenient way to create workflows. Backends are automatically registered
based on enabled crate features.

## Backend Auto-Registration

When a `Quillmark` engine is created with [`Quillmark::new`], it automatically
registers all backends based on enabled features:

- **typst** (default) - Typst backend for PDF/SVG rendering

## Workflow

1. Create an engine with [`Quillmark::new`]
2. Register quills with [`register_quill`](Quillmark::register_quill)
3. Load workflows with [`load`](Quillmark::load)
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
