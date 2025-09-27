# QuillMark

A flexible markdown rendering library with pluggable backends.

## Overview

QuillMark provides a trait-based architecture for rendering markdown to various output formats using different backends. The library is designed to be extensible and supports multiple backends that can be enabled through feature flags.

## Crates

- **`quillmark`**: The core library that defines the traits and common types
- **`quillmark-typst`**: A backend implementation using Pandoc and Typst for PDF and SVG generation

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
quillmark = "0.1"
quillmark-typst = "0.1"  # Optional: for Typst backend support
```

Basic usage:

```rust
use quillmark::{render, Options, OutputFormat};

let options = Options {
    backend: Some("typst".to_string()),
    format: Some(OutputFormat::Pdf),
};

let artifacts = render("# Hello World", &options)?;
```

## Architecture

The library is built around the `Backend` trait:

```rust
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn render(&self, markdown: &str, opts: &Options) -> Result<Vec<Artifact>, RenderError>;
}
```

Backends can produce multiple artifacts (e.g., multi-page documents) in various formats.

## Development

To build and test all crates:

```bash
cargo test --all
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.