# Typst Backend for Quillmark

This crate provides a complete Typst backend implementation that converts Markdown
documents to PDF and SVG formats via the Typst typesetting system.

## Overview

The primary entry point is the [`TypstBackend`] struct, which implements the
[`Backend`] trait from `quillmark-core`. Users typically interact with this backend
through the high-level `Workflow` API from the `quillmark` crate.

## Features

- Converts CommonMark Markdown to Typst markup
- Compiles Typst documents to PDF and SVG formats
- Provides template filters for YAML data transformation
- Manages fonts, assets, and packages dynamically
- Thread-safe for concurrent rendering

## Example Usage

```rust,no_run
use quillmark_typst::TypstBackend;
use quillmark_core::{Backend, Quill, OutputFormat};

let backend = TypstBackend::default();
let quill = Quill::from_path("path/to/quill").unwrap();

// Use with Workflow API (recommended)
// let workflow = Workflow::new(Box::new(backend), quill);
```
## Modules

- [`convert`] - Markdown to Typst conversion utilities
- [`compile`] - Typst to PDF/SVG compilation functions
