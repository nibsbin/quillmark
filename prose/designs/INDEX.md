# Quillmark Design Documentation Index

This directory contains design documents that describe the architecture and components of Quillmark. These documents are the primary reference for agents implementing features and maintaining the codebase.

## Core Architecture

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Overall system architecture, crate structure, and core design principles
- **[ERROR.md](ERROR.md)** - Error handling system with structured diagnostics and cross-language serialization

## Core Components

- **[PARSE.md](PARSE.md)** - Markdown parsing and Extended YAML Metadata Standard
- **[QUILL.md](QUILL.md)** - Quill template bundle structure, file tree API, and JSON contract
- **[QUILL_VALUE.md](QUILL_VALUE.md)** - Unified value type for TOML/YAML/JSON conversions
- **[SCHEMAS.md](SCHEMAS.md)** - Configuration schemas and field validation
- **[CARDS.md](CARDS.md)** - Composable cards architecture with unified CARDS array
- ~~**[SCOPES.md](SCOPES.md)**~~ - *Superseded by CARDS.md*
- **[TEMPLATE_DRY_RUN.md](TEMPLATE_DRY_RUN.md)** - Template dry run for lightweight validation
- **[GLUE_METADATA.md](GLUE_METADATA.md)** - Template metadata access via `__metadata__` field
- **[DEFAULT_QUILL.md](DEFAULT_QUILL.md)** - Default quill system for zero-config rendering

## Backends

- **[ACROFORM.md](ACROFORM.md)** - AcroForm backend for PDF form filling
- **[TYPST_GUILLEMET_CONVERSION.md](TYPST_GUILLEMET_CONVERSION.md)** - Guillemet conversion for Typst backend
- Typst backend documentation is in the rustdoc (see `crates/backends/typst/`)

## Language Bindings

- **[CLI.md](CLI.md)** - Command-line interface design and implementation
- **[PYTHON.md](PYTHON.md)** - Python bindings via PyO3
- **[WASM.md](WASM.md)** - WebAssembly bindings for JavaScript/TypeScript

## Infrastructure

- **[CI_CD.md](CI_CD.md)** - Continuous integration and delivery workflows

## Using These Documents

### For Architect Agents

When creating new designs or updating existing ones:

1. **Check existing designs first** - Review related documents to maintain consistency
2. **Follow DRY** - Cross-reference instead of duplicating information
3. **Update, don't replace** - Prefer updating existing designs over creating new ones
4. **No code** - Designs describe desired state, not implementation details
5. **Focus on "what" and "why"** - Not "how" or "when"

### For Implementation

- Designs describe the **desired state** of the system
- Implementation details live in rustdoc (run `cargo doc --open`)
- For current implementation status, check the **Status** field at the top of each document

### Cross-References

Design documents frequently reference each other. Key relationships:

- ARCHITECTURE → all component designs
- PARSE → GLUE_METADATA (metadata structure)
- QUILL → SCHEMAS (field definitions)
- ERROR → all bindings (error handling patterns)
- DEFAULT_QUILL → QUILL, PARSE (template selection)

## Status Indicators

Each design document includes a status indicator:

- **Design Phase** - Planned but not yet implemented
- **Implemented** - Fully implemented and production-ready
- **Final Design** - Locked specification, no backward compatibility planned

## Debrief Documents

The `prose/debriefs/` directory contains post-implementation learnings and may include insights not yet integrated into these design documents.
