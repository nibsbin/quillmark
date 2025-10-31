---
name: Programmer
description: Translates design documents and plans into code
---

# Programmer Agent

- Design documents in `prose/designs/` are authoritative
- Move the original plan to `prose/debriefs/` and update with implementation status
- Practice KISS and DRY

## Architecture Overview

**Template-first Markdown rendering**: Parse (Markdown + YAML) → Template (MiniJinja) → Compile (backend glue) → Artifacts (PDF/SVG)

**Workspace structure** (publish order `core` → `typst` → `quillmark`, synchronized versions):
- `quillmark-core/` - Foundation (parsing, templating, Backend trait)
- `backends/quillmark-typst/` - Typst backend implementation
- `backends/quillmark-acroform/` - PDF Acroform backend implementation
- `quillmark/` - Orchestration layer (Quillmark engine and workflows)
- `quillmark-fixtures/` - Test resources
- `quillmark-fuzz/` - Fuzzing tests
- `bindings/quillmark-python/` - Python bindings
- `bindings/quillmark-wasm/` - WASM bindings

See `designs/ARCHITECTURE.md` for complete architecture.

## Code Documentation Strategy

- Use standard in-line Rust doc comments (`///`)
- Only create minimal examples for public APIs
- Err on the side of brevity
- Avoid documentation creep; keep docs focused and up-to-date

## Design Document Philosophy
All design documents in `designs/` should follow consistent principles:

- High-level only - Focus on architecture, not implementation
- Minimal code - Only essential examples, reference actual code
- Medium detail - Enough to understand, not enough to implement
- KISS - Keep it simple and maintainable
- References - Point to actual implementation for details

## Implementation/Testing Strategy

- This is pre-1.0 software. Never worry about backwards compatibility. Actively remove legacy code/comments.