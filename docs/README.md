# Quillmark Documentation

This directory contains the documentation for Quillmark.

## Building Documentation

Install dependencies:

```bash
pip install -r docs-requirements.txt
```

Build the documentation:

```bash
mkdocs build
```

Serve locally for development:

```bash
mkdocs serve
```

The documentation will be available at http://127.0.0.1:8000

## Structure

- `index.md` - Homepage with project overview
- `getting-started/` - Installation and basic concepts
  - `quickstart.md` - Quick installation for Python, Rust, and JavaScript
  - `concepts.md` - Core concepts and mental models
- `guides/` - Language-agnostic guides
  - `creating-quills.md` - How to create Quill templates
  - `quill-markdown.md` - Markdown syntax and frontmatter
  - `typst-backend.md` - Typst backend guide
  - `acroform-backend.md` - AcroForm backend guide
- `python/` - Python-specific documentation
  - `api.md` - Python API reference

## Links

- [Rust API Documentation](https://docs.rs/quillmark/latest/quillmark/)
- [GitHub Repository](https://github.com/nibsbin/quillmark)
