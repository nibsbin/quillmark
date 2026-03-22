# Default Quill System

> **Status**: Implemented

## Overview

When `ParsedDocument::from_markdown()` parses markdown without a `QUILL:` directive, it sets `quill_reference = QuillReference::latest("__default__")`. If `__default__` is not registered, the engine emits:

```
Quill '__default__' not registered.
Add `QUILL: <name>` to the markdown frontmatter or register a default Quill.
```

## Backend Trait

```rust
pub trait Backend: Send + Sync {
    fn default_quill(&self) -> Option<Quill> { None }
}
```

`Quillmark::register_backend()` calls `default_quill()` and registers the result as `__default__` if no default is already registered.

## Typst Backend

The Typst backend provides an embedded default Quill at `backends/quillmark-typst/default_quill/` (name `__default__`, embedded at compile time via `include_str!`/`include_bytes!`).

## Name Reservation

`__default__` is reserved. Manual registration of `__default__` is allowed but must pass backend/plate validation. Duplicate names are always errors; default Quills are only auto-registered when `__default__` is absent.

## Error Handling

| Condition | Error Type | Code |
|-----------|-----------|------|
| No QUILL field, no default registered | `RenderError::UnsupportedBackend` | `engine::missing_quill_tag` |
| Backend default Quill fails validation | `RenderError::QuillConfig` (warning, registration continues) | — |
