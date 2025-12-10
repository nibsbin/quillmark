# Quillmark

[![Crates.io](https://img.shields.io/crates/v/quillmark.svg)](https://crates.io/crates/quillmark)
[![PyPI](https://img.shields.io/pypi/v/quillmark.svg?color=3776AB)](https://pypi.org/project/quillmark/)
[![npm](https://img.shields.io/npm/v/@quillmark-test/wasm.svg?color=CB3837)](https://www.npmjs.com/package/@quillmark-test/wasm)
[![CI](https://github.com/nibsbin/quillmark/workflows/CI/badge.svg)](https://github.com/nibsbin/quillmark/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-lightgray.svg)](LICENSE)

A template-first Markdown rendering system that converts Markdown with YAML frontmatter into PDF, SVG, and other output formats.

Maintained by [tonguetoquill.com](https://www.tonguetoquill.com).

**UNDER DEVELOPMENT**

## Features

- **Template-first design**: Quill templates control structure and styling, Markdown provides content
- **YAML metadata**: Extended YAML support for inline metadata blocks
- **Multiple backends**: 
  - PDF and SVG output via Typst backend
  - PDF form filling via AcroForm backend
- **Structured error handling**: Clear diagnostics with source locations
- **Dynamic asset loading**: Fonts, images, and packages resolved at runtime

## Documentation

- **[User Guide](https://quillmark.readthedocs.io)** - Tutorials, concepts, and language bindings
- **[Rust API Reference](https://docs.rs/quillmark)** - for the Quillmark crate

## Installation

Add Quillmark to your `Cargo.toml`:

```bash
cargo add quillmark
```

## Quick Start

```rust
use quillmark::{Quillmark, OutputFormat, ParsedDocument};
use quillmark_core::Quill;

// Create engine with Typst backend
let mut engine = Quillmark::new();

// Load a quill template
let quill = Quill::from_path("path/to/quill")?;
engine.register_quill(quill);

// Parse markdown once
let markdown = "---\ntitle: Example\n---\n\n# Hello World";
let parsed = ParsedDocument::from_markdown(markdown)?;

// Load workflow and render to PDF
let workflow = engine.workflow("quill_name")?;
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;

// Access the generated PDF
let pdf_bytes = &result.artifacts[0].bytes;
```

## Examples

Run the included examples:

```bash
cargo run --example appreciated_letter
cargo run --example usaf_memo
cargo run --example taro
cargo run --example usaf_form_8
cargo run --example auto_plate
cargo run --example test_defaults
```

## Documentation

- [API Documentation](https://docs.rs/quillmark)
- [Architecture Design](prose/designs/ARCHITECTURE.md)
- [Contributing Guide](CONTRIBUTING.md)

## Project Structure

This workspace contains:

- **crates/core** - Core parsing, templating, and backend traits
- **crates/quillmark** - High-level orchestration API
- **crates/backends/typst** - Typst backend for PDF/SVG output
- **crates/backends/acroform** - AcroForm backend for PDF form filling
- **crates/bindings/python** - Python bindings (PyO3)
- **crates/bindings/wasm** - WebAssembly bindings
- **crates/bindings/cli** - Command-line interface
- **crates/fixtures** - Test fixtures and utilities
- **crates/fuzz** - Fuzz testing suite

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
