# Quillmark

[![Crates.io](https://img.shields.io/crates/v/quillmark.svg)](https://crates.io/crates/quillmark)
[![PyPI](https://img.shields.io/pypi/v/quillmark.svg?color=3776AB)](https://pypi.org/project/quillmark/)
[![npm](https://img.shields.io/npm/v/@quillmark/wasm.svg?color=CB3837)](https://www.npmjs.com/package/@quillmark/wasm)
[![Documentation](https://docs.rs/quillmark/badge.svg)](https://docs.rs/quillmark)

**Quillmark is a schema-driven document engine.** A Quill declares a document format as data — a typed schema plus the presentation that renders it — and the engine turns a document conforming to that schema into a typeset artifact (PDF, SVG, PNG) through the Typst backend, or into a stamped interactive AcroForm PDF through the `pdfform` backend.

A document is a structured value, not text. Markdown, the annotated blueprint an LLM fills in, and programmatic construction are three projections of it; all three produce the same `Document`, and it is validated against the same schema whichever wrote it.

!!! warning "Unstable APIs"
    APIs may change between releases. Every break is covered by a guide under Migration.

## Choose your path

- **Writing documents?** You author against a Quill format, in Markdown.  
  → [Markdown Syntax](authoring/markdown-syntax.md)

- **Building quills?** You create Quill formats that control rendering.  
  → [Creating Quills](quills/creating-quills.md)

- **Integrating into an app?** You use Quillmark via Python or JavaScript.  
  → [Quickstart](getting-started/quickstart.md) · [Integrating Quillmark](integration/index.md)

- **Using the CLI?** You render and validate from the command line.  
  → [CLI Reference](cli/reference.md)

- **Using Rust?** API documentation is on [docs.rs](https://docs.rs/quillmark).
