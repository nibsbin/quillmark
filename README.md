# Quillmark

[![Crates.io](https://img.shields.io/crates/v/quillmark.svg)](https://crates.io/crates/quillmark)
[![Documentation](https://docs.rs/quillmark/badge.svg)](https://docs.rs/quillmark)
[![CI](https://github.com/nibsbin/quillmark/workflows/CI/badge.svg)](https://github.com/nibsbin/quillmark/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

A template-first Markdown rendering system that converts Markdown with YAML frontmatter into PDF, SVG, and other output formats.

## Features

- **Template-first design**: Quill templates control structure and styling, Markdown provides content
- **YAML frontmatter support**: Extended YAML metadata with inline sections
- **Multiple backends**: PDF and SVG output via Typst backend
- **Structured error handling**: Clear diagnostics with source locations
- **Dynamic asset loading**: Fonts, images, and packages resolved at runtime

## Installation

### Library

Add Quillmark to your `Cargo.toml`:

```bash
cargo add quillmark
```

### Command-Line Tool

Install the CLI tool:

```bash
cargo install --path quillmark-cli
```

Or use it directly:

```bash
quillmark-cli <quill_path> <markdown_file>
```

See [quillmark-cli/README.md](quillmark-cli/README.md) for CLI documentation.

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
let workflow = engine.workflow_from_quill_name("quill_name")?;
let result = workflow.render(&parsed, Some(OutputFormat::Pdf))?;

// Access the generated PDF
let pdf_bytes = &result.artifacts[0].bytes;
```

## Examples

### Library Examples

Run the included examples:

```bash
cargo run --example appreciated_letter
cargo run --example usaf_memo
cargo run --example taro
```

### CLI Examples

Render a markdown file using a quill template:

```bash
quillmark-cli quillmark-fixtures/resources/taro document.md
```

Specify a custom output path:

```bash
quillmark-cli quillmark-fixtures/resources/taro document.md -o output.pdf
```

## Documentation

- [API Documentation](https://docs.rs/quillmark)
- [Architecture Design](designs/DESIGN.md)
- [Contributing Guide](CONTRIBUTING.md)

## Project Structure

This workspace contains:

- **quillmark-core** - Core parsing, templating, and backend traits
- **quillmark** - High-level orchestration API
- **quillmark-typst** - Typst backend for PDF/SVG output
- **quillmark-cli** - Command-line interface for rendering
- **quillmark-fixtures** - Test fixtures and utilities

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
