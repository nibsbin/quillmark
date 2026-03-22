# Quillmark Design Index

## Core

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Crate structure and system overview
- **[ERROR.md](ERROR.md)** - Structured diagnostics and cross-language serialization

## Components

- **[PARSE.md](PARSE.md)** - Markdown parsing and Extended YAML Metadata Standard
- **[EXTENDED_MARKDOWN.md](EXTENDED_MARKDOWN.md)** - Extended Markdown format specification
- **[QUILL.md](QUILL.md)** - Quill bundle structure, file tree API, and JSON contract
- **[QUILL_VALUE.md](QUILL_VALUE.md)** - Unified value type for TOML/YAML/JSON conversions
- **[VERSIONING.md](VERSIONING.md)** - Quill version resolution
- **[SCHEMAS.md](SCHEMAS.md)** - Field schema and validation
- **[CARDS.md](CARDS.md)** - Composable cards with unified CARDS array
- ~~**[SCOPES.md](SCOPES.md)**~~ - *Superseded by CARDS.md*
- **[TEMPLATE_DRY_RUN.md](TEMPLATE_DRY_RUN.md)** - Dry run validation
- **[GLUE_METADATA.md](GLUE_METADATA.md)** - Plate data injection
- **[DEFAULT_QUILL.md](DEFAULT_QUILL.md)** - Default quill for zero-config rendering

## Backends

- **[ACROFORM.md](ACROFORM.md)** - AcroForm backend
- Typst backend: see `crates/backends/typst/` rustdoc

## Bindings

- **[CLI.md](CLI.md)** - Command-line interface
- **[PYTHON.md](PYTHON.md)** - Python bindings (PyO3)
- **[WASM.md](WASM.md)** - WebAssembly bindings

## Infrastructure

- **[CI_CD.md](CI_CD.md)** - CI/CD workflows
