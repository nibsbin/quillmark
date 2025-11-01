# Default Quill System

> **Status**: Design Document
>
> This document defines the architecture and design for the default Quill system in Quillmark.

---

## Overview

The default Quill system provides a fallback Quill template when users do not specify a `QUILL:` tag in their markdown frontmatter. This enhances usability by allowing simple documents to be rendered without requiring explicit Quill selection.

## Design Principles

1. **Reserved Naming Convention** - Default Quills use the reserved name `__default__` to avoid collisions with user-defined Quills
2. **Backend Ownership** - Each backend can optionally provide a default Quill implementation
3. **Explicit Registration** - Default Quills are registered during backend registration if no default already exists
4. **Graceful Fallback** - When no `QUILL:` tag is specified, the engine uses `__default__` if available
5. **Clear Error Messages** - If neither a Quill tag nor default Quill is available, emit actionable errors

---

## Architecture

### Backend Trait Extension

The `Backend` trait is extended with a new optional method:

```rust
pub trait Backend: Send + Sync {
    // ... existing methods ...
    
    /// Provide an embedded default Quill for this backend.
    /// 
    /// Returns `None` if the backend does not provide a default Quill.
    /// The returned Quill will be registered with the name `__default__`
    /// during backend registration if no default Quill already exists.
    fn default_quill(&self) -> Option<Quill> {
        None
    }
}
```

**Design Rationale:**
- Default implementation returns `None` to maintain backward compatibility
- Backends can opt-in by implementing this method
- The engine handles registration, not the backend

### Engine-Level Registration

The `Quillmark::register_backend()` method is enhanced to:

1. Register the backend as usual
2. Check if a default Quill (`__default__`) already exists
3. If not, call `backend.default_quill()` to get an optional default Quill
4. If a default Quill is returned, register it with the name `__default__`

**Implementation Location:** `quillmark/src/orchestration.rs::Quillmark::register_backend()`

### Workflow Loading Enhancement

The `Quillmark::workflow_from_parsed()` method loads workflows from a ParsedDocument:

1. The parsed document always has a quill_tag (either from QUILL: directive or `__default__`)
2. The workflow is loaded using the quill_tag directly
3. If the quill is not registered, emit a clear error message

**Parse-Time Default Assignment:**

When `ParsedDocument::from_markdown()` parses markdown without a QUILL: directive, it sets `quill_tag = "__default__"` at parse time. This ensures:
- ParsedDocument.quill_tag is never None (non-optional String field)
- Consumers (WASM, Python bindings) don't need to implement default quill logic
- The contract is cleaner: every parsed document has a quill tag

**Error Message Pattern:**
```
Quill '__default__' not registered.
Add `QUILL: <name>` to the markdown frontmatter or register a default Quill.
```

---

## Typst Backend Default Quill

The Typst backend already has a default Quill implementation located at:
- **Path:** `backends/quillmark-typst/default_quill/`
- **Name:** `__default__` (as defined in `Quill.toml`)
- **Glue File:** `glue.typ` (minimal template with metadata and body)
- **Example:** `example.md`

### Implementation

The `TypstBackend` implements the `default_quill()` method:

```rust
impl Backend for TypstBackend {
    // ... existing methods ...
    
    fn default_quill(&self) -> Option<Quill> {
        // Load embedded default Quill from default_quill/ directory
        // embedded at compile time using include_str!/include_bytes!
        Some(create_embedded_default_quill())
    }
}
```

The default Quill is embedded at compile time to avoid filesystem dependencies.

---

## Name Reservation

The name `__default__` is reserved for default Quills:

1. **Validation:** `Quillmark::register_quill()` should emit a warning or error if users attempt to register a Quill named `__default__` manually
2. **Exception:** Only backend-provided default Quills can use this name
3. **Pattern:** Double underscore prefix/suffix follows Rust/Python conventions for reserved names

---

## Test Retrofitting Strategy

Tests that don't require specific Quill features should be updated to:

1. Remove explicit Quill registration where not needed
2. Remove `QUILL:` tags from test markdown
3. Rely on the default Quill for simpler test cases
4. Keep explicit Quills for tests that need specific features

**Benefits:**
- Simpler test code
- Better coverage of default Quill behavior
- Easier maintenance

**Approach:**
- Review tests in `quillmark/tests/`
- Identify tests using minimal/generic Quills
- Refactor to use default Quill
- Ensure tests still validate intended behavior

---

## Error Handling

### No Quill Specified and No Default Available

**Error Type:** `RenderError::UnsupportedBackend`

**Message:**
```
No QUILL field found in parsed document and no default Quill is registered.
```

**Code:** `engine::missing_quill_tag`

**Hint:**
```
Add 'QUILL: <name>' to specify which Quill template to use, or ensure a backend with a default Quill is registered.
```

### Default Quill Registration Failure

If a backend's default Quill fails validation during registration:

**Error Type:** `RenderError::QuillConfig`

**Action:** Log warning and continue without registering the default Quill

**Rationale:** Backend registration should not fail due to invalid default Quill

---

## Future Considerations

1. **Multiple Default Quills:** Could support backend-specific defaults
2. **Configuration:** Allow users to override the default Quill name
3. **Per-Backend Fallback:** If no global default, fall back to backend-specific defaults
4. **Default Quill Discovery:** Load default Quills from a standard directory

These are not part of the current design but may be considered in future iterations.
