# Quillmark Core Overview

Core types and functionality for the Quillmark template-first Markdown rendering system.

## Features

This crate provides the foundational types and traits for Quillmark:

- **Parsing**: YAML frontmatter extraction with Extended YAML Metadata Standard support
- **Templating**: MiniJinja-based template composition with stable filter API
- **Template model**: `Quill` type for managing template bundles with in-memory file system
- **Backend trait**: Extensible interface for implementing output format backends
- **Error handling**: Structured diagnostics with source location tracking
- **Utilities**: TOML⇄YAML conversion helpers

## Quick Start

```rust,no_run
use quillmark_core::{decompose, Quill};

// Parse markdown with frontmatter
let markdown = "---\ntitle: Example\n---\n\n# Content";
let doc = decompose(markdown).unwrap();

// Load a quill template
let quill = Quill::from_path("path/to/quill").unwrap();
```

## Architecture

The crate is organized into four main modules:

- **`parse`**: Markdown parsing with YAML frontmatter support
- **`templating`**: Template composition using MiniJinja
- **`backend`**: Backend trait for output format implementations
- **`error`**: Structured error handling and diagnostics

## Further Reading

- [API.md](../API.md) - Comprehensive API reference
- [PARSE.md](../PARSE.md) - Detailed parsing documentation
- [Examples](../../examples/) - Working examples
