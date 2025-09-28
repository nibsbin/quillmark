# QuillMark

A flexible markdown rendering library with pluggable backends.

## Overview

QuillMark provides a trait-based architecture for rendering markdown to various output formats using different backends. The library is designed to be extensible and supports multiple backends that can be enabled through feature flags.

## Crates

- **`quillmark-core`**: Core types and traits shared between all components
- **`quillmark`**: The main library that provides the high-level API and re-exports core types
- **`quillmark-typst`**: A backend implementation using `pulldown-cmark` and Typst for PDF and SVG generation

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
quillmark = "0.1"
quillmark-typst = "0.1"  # Optional: for Typst backend support
```

Basic usage (backends are passed directly in the render configuration):

```rust
use quillmark::{render, RenderConfig};
use quillmark_typst::TypstBackend;
use quillmark_core::OutputFormat;

let backend = TypstBackend::new();
let config = RenderConfig {
    backend: Box::new(backend),
    output_format: Some(OutputFormat::Pdf),
    quill_path: None, // or Some(path) to a quill template directory
};

let artifacts = render("# Hello World", &config)?;
```

## Architecture

The library is built around the `Backend` trait defined in `quillmark-core`:

```rust
pub trait Backend: Send + Sync {
    fn id(&self) -> &'static str;
    fn supported_formats(&self) -> &'static [OutputFormat];
    fn render(&self, markdown: &str, opts: &Options) -> Result<Vec<Artifact>, RenderError>;
}
```

The crate structure avoids cyclical dependencies:
- `quillmark-core` contains shared types and traits
- `quillmark` depends on `quillmark-core` and provides the main API
- Backend crates like `quillmark-typst` depend only on `quillmark-core`

Backends can produce multiple artifacts (e.g., multi-page documents) in various formats.

Note: The previous global backend registration API (a global registry and `register_backend`) has been removed. Backends are now supplied directly in `RenderConfig`, which makes backend selection explicit and avoids global mutable state.

## Package Management (Typst Backend)

The `quillmark-typst` backend features a simplified, efficient dynamic package loading system that replaces previous hardcoded approaches:

### Key Features

- **Dynamic Discovery**: Automatically discovers packages in `{quill}/packages/` directories
- **Proper Path Handling**: Maintains directory structure (e.g., `src/lib.typ` imports work correctly)
- **Entrypoint Support**: Reads `typst.toml` files to respect package entrypoint configurations
- **Namespace Support**: Handles `@preview`, `@local`, and custom namespaces
- **Asset Management**: Loads assets from `{quill}/assets/` with correct virtual paths

### Package Structure

```
hello-quill/
├── packages/
│   └── tonguetoquill-usaf-memo/
│       ├── typst.toml          # Package metadata
│       └── src/
│           ├── lib.typ         # Main entrypoint
│           └── utils.typ       # Supporting files
├── assets/
│   ├── fonts/
│   └── dod_seal.gif           # Accessible as "assets/dod_seal.gif"
└── glue.typ                   # Template file
```

### Usage in Templates

```typst
#import "@preview/tonguetoquill-usaf-memo:0.1.1": official-memorandum
#show:official-memorandum.with(
  letterhead-seal: image("assets/dod_seal.gif"),
  // ... other parameters
)
```

The system automatically resolves package imports and asset references without manual configuration.

## Development

To build and test all crates:

```bash
cargo test --all
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.