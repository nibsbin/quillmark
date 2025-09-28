# QuillMark Fixtures

This crate provides centralized test fixtures and utilities for QuillMark examples and tests.

## Overview

The `quillmark-fixtures` crate consolidates all example resources (sample markdown files, quill templates, assets, etc.) and provides utilities for writing outputs to standardized locations.

## Usage

### Accessing Resources

Use `resource_path()` to get paths to fixture resources:

```rust
use quillmark_fixtures::resource_path;

let sample_md = resource_path("sample.md")?;
let hello_quill_template = resource_path("hello-quill")?;
```

### Writing Example Outputs

Use `write_example_output()` for convenience or `example_output_dir()` for more control:

```rust
use quillmark_fixtures::{write_example_output, example_output_dir};

// Convenience function
let output_path = write_example_output("hello-quill", "output.pdf", &pdf_bytes)?;

// Or get the directory and write manually
let output_dir = example_output_dir("hello-quill")?;
std::fs::write(output_dir.join("output.pdf"), &pdf_bytes)?;
```

All outputs will be written to `target/examples/<example-name>/...` in the workspace root.

## Directory Structure

```
quillmark-fixtures/
├── resources/           # All example resources
│   ├── sample.md       # Sample markdown files
│   ├── hello-quill/    # Quill template directories
│   │   ├── glue.typ    # Template files
│   │   ├── assets/     # Font and image assets
│   │   └── packages/   # Typst packages
│   └── simple-quill/   # More templates
└── src/lib.rs          # Utility functions
```

## Migration from test_context

The old `test_context` module is deprecated but still works for backward compatibility. New code should use `quillmark-fixtures` directly:

- `test_context::examples_dir()` → `quillmark_fixtures::resource_path("")`
- `test_context::examples_path(path)` → `quillmark_fixtures::resource_path(path)`
- `test_context::create_output_dir(name)` → `quillmark_fixtures::example_output_dir(name)`