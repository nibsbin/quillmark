# Error Handling System

Quillmark's error handling system provides structured diagnostics with source location tracking for actionable error reporting.

> **Implementation**: See `quillmark-core/src/error.rs` for complete API documentation

## Overview

The error handling system is designed to:

* Preserve **source location** information (file, line, column) where available
* Maintain **machine-readable** and **pretty-printable** diagnostics
* Provide **helpful hints** for common error scenarios
* Support **error chaining** to preserve context through the stack
* Enable **cross-language serialization** for Python and WASM bindings

## Core Architecture

### Error Types

**`Severity`**: Error level classification
- `Error` - Fatal errors that prevent completion
- `Warning` - Non-fatal issues that may need attention
- `Note` - Informational messages

**`Location`**: Source position tracking with file name, line number (1-indexed), and column number (1-indexed)

**`Diagnostic`**: Structured error information containing:
- Severity level
- Optional error code (e.g., "E001", "typst::syntax")
- Human-readable message
- Primary source location
- Optional hint for fixing the error
- Source error chain for error propagation

**`RenderError`**: Main error enum for rendering operations with variants for:
- Engine creation failures
- Invalid frontmatter
- Template rendering errors
- Backend compilation failures (may contain multiple diagnostics)
- Format/backend support errors
- Resource collision errors
- Size limit violations

**`SerializableDiagnostic`**: Cross-language compatible version of `Diagnostic` with flattened source chain for Python and WASM bindings

## Backend Error Mapping

Backends convert their native error types to Quillmark's structured diagnostics.

### Template Engine (MiniJinja)

MiniJinja errors are automatically converted to `RenderError::TemplateFailed` with:
- Accurate line/column extraction from error location
- Context-aware hints based on error kind (undefined variables, syntax errors, etc.)
- Preserved error chain via `source` field
- Structured error codes (e.g., "minijinja::UndefinedError")

### Typst Backend

Typst diagnostics are mapped via `map_typst_errors()`:
- Severity levels mapped appropriately (Error/Warning)
- Spans resolved to precise file/line/column locations
- Hints preserved from Typst errors
- Error codes in format "typst::error_type" (e.g., "typst::unknown_variable")

See `backends/quillmark-typst/src/error_mapping.rs` for implementation details.

## Error Presentation

**Pretty Printing**: The `Diagnostic::fmt_pretty()` method provides human-readable output:
```
[ERROR] Undefined variable (E001) at template.typ:10:5
  hint: Check variable spelling
```

**Consolidated Printing**: The `print_errors()` helper handles all `RenderError` variants and formats them consistently.

**Machine-Readable**: All diagnostic types implement `serde::Serialize` for JSON export and tooling integration.

## Best Practices

### For Backend Implementors

1. **Return structured diagnostics** - Convert native backend errors to `Diagnostic` objects with appropriate severity, location, and hints
2. **Provide actionable hints** - Match common error patterns and suggest concrete fixes
3. **Use consistent error codes** - Format as `backend::error_type` (e.g., "typst::unknown_variable")
4. **Map source locations** - Extract precise file/line/column information when available
5. **Test error paths** - Verify error messages are helpful and locations are accurate

### For Application Developers

1. **Use `print_errors()` for CLI** - Provides consistent formatting across all error types
2. **Handle `Result` types explicitly** - Never unwrap rendering results; use proper error handling
3. **Check warnings** - Inspect `RenderResult.warnings` and display to users
4. **Serialize for tooling** - All diagnostic types support JSON serialization via `serde`
5. **Preserve error context** - Use error chaining to maintain full diagnostic information

---

## Cross-References

**Related Design Documents:**
- [ARCHITECTURE.md](ARCHITECTURE.md) - Error handling patterns in orchestration workflow
- [PARSE.md](PARSE.md) - Parse error handling and validation
- [DEFAULT_QUILL.md](DEFAULT_QUILL.md) - Error handling when no quill is available

**Implementation:**
- `quillmark-core/src/error.rs` - Core error types and diagnostics
- `quillmark/src/orchestration.rs` - Error propagation in workflow

**Language Bindings:**
- [PYTHON.md](PYTHON.md) - Python error mapping
- [WASM.md](WASM.md) - WebAssembly error serialization
