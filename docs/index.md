# Quillmark

[![Crates.io](https://img.shields.io/crates/v/quillmark.svg)](https://crates.io/crates/quillmark)
[![PyPI](https://img.shields.io/pypi/v/quillmark.svg?color=3776AB)](https://pypi.org/project/quillmark/)
[![npm](https://img.shields.io/npm/v/@quillmark-test/wasm.svg?color=CB3837)](https://www.npmjs.com/package/@quillmark-test/wasm)
[![Documentation](https://docs.rs/quillmark/badge.svg)](https://docs.rs/quillmark)

A template-first Markdown rendering system that converts Markdown with YAML frontmatter into PDF, SVG, and other output formats.

!!! warning "Under Development"
    This project is under active development and APIs may change.

## Features

- **Template-first design**: Quill templates control structure and styling, Markdown provides content
- **YAML frontmatter support**: Extended YAML metadata with inline sections
- **Multiple backends**: 
  - PDF and SVG output via Typst backend
  - PDF form filling via AcroForm backend
- **Structured error handling**: Clear diagnostics with source locations
- **Dynamic asset loading**: Fonts, images, and packages resolved at runtime

## Quick Links

- [Quickstart Guide](getting-started/quickstart.md) - Get up and running in minutes
- [Concepts](getting-started/concepts.md) - Understand Quillmark's design
- [Rust API Documentation](https://docs.rs/quillmark/latest/quillmark/) - Complete Rust API reference
- [GitHub Repository](https://github.com/nibsbin/quillmark)

## Project Structure

Quillmark is organized as a workspace with multiple crates:

- **quillmark-core** - Core parsing, templating, and backend traits
- **quillmark** - High-level orchestration API
- **backends/quillmark-typst** - Typst backend for PDF/SVG output
- **backends/quillmark-acroform** - AcroForm backend for PDF form filling
- **bindings/quillmark-python** - Python bindings (PyO3)
- **bindings/quillmark-wasm** - WebAssembly bindings for JavaScript

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](https://github.com/nibsbin/quillmark/blob/main/LICENSE) for details.
