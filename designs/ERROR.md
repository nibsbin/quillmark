# Error Handling System Documentation

**Status:** Phase 1 Complete  
**Scope:** quillmark, quillmark-core, quillmark-typst

> **For complete implementation details, see**:
> - `quillmark-core/src/error.rs` - Core error types with API documentation
> - `quillmark-typst/src/error_mapping.rs` - Typst error mapping
> - `quillmark-typst/src/compile.rs` - Compilation error handling

---

## Overview

Quillmark uses a **structured error handling strategy** that:

* Preserves **line/column** and **source file** information where available
* Keeps diagnostics **machine-readable** and **pretty-printable**
* Avoids stringly-typed errors
* Provides helpful hints for common error scenarios
* **Maintains error source chains** through the `source` field

---

## Core Architecture

### Error Types

1. **`Severity`** - Error level classification (Error, Warning, Note)

2. **`Location`** - Source position tracking (file, line, column)

3. **`Diagnostic`** - Structured error information with severity, code, message, location, hint, and source chain

### SerializableDiagnostic

For cross-language boundaries (Python, WASM), a serializable version with flattened source chain is provided.

### RenderError Enum

All rendering errors are represented by the `RenderError` enum, with every variant containing Diagnostic payloads:
- `EngineCreation`, `InvalidFrontmatter`, `TemplateFailed` contain single diagnostic
- `CompilationFailed` may contain multiple diagnostics
- All variants provide structured error information

---

## Error Source Mapping

### MiniJinja (Template Engine)

The `From<minijinja::Error> for RenderError` implementation:
- Captures line number and column position
- Preserves original error via `with_source()`
- Generates context-aware hints based on error kind
- Creates structured `Diagnostic` with error code

### Typst (Backend Compiler)

The `map_typst_errors()` function converts Typst diagnostics:
- Maps severity levels appropriately
- Resolves spans to precise file/line/column locations
- Preserves hints from Typst errors
- Generates error codes like `typst::unknown_variable`

---

## Error Printing

### Pretty Printing

The `Diagnostic::fmt_pretty()` method provides human-readable output with severity labels, error codes, locations, and hints.

### Consolidated Error Printing

The `print_errors()` function handles all `RenderError` variants and formats them consistently.

---

## Implementation Status

### ✅ Phase 1: Core Diagnostic System (Complete)

1. **Source Chain Support** ✅
   - Added optional `source` field to Diagnostic
   - Implemented `with_source()` builder method
   - Implemented `source_chain()` to extract error chain
   - Implemented `fmt_pretty_with_source()` for debugging

2. **SerializableDiagnostic** ✅
   - Created for cross-language boundaries (Python, WASM)
   - Includes flattened `source_chain` instead of trait object
   - Implements From<Diagnostic> and From<&Diagnostic>

3. **Standardized Error Variants** ✅
   - All RenderError variants now use Diagnostic payloads
   - Removed legacy Internal, Other, and Template variants
   - Consistent error structure across all error types

4. **Backend Safety** ✅
   - Removed `.unwrap()` from PDF compilation
   - Changed return types to `Result<_, RenderError>`
   - Proper error propagation via structured Diagnostics

5. **Error Preservation** ✅
   - MiniJinja errors preserve source chain
   - Typst errors properly converted to Diagnostics
   - All error conversions maintain context

6. **Cross-Language Support** ✅
   - Python bindings updated to use SerializableDiagnostic
   - WASM bindings updated to use SerializableDiagnostic
   - Error information properly propagated across FFI

**Testing:** All 87 tests passing (54 unit + 33 integration/doc)

### ⚠️ Phase 2: Enhanced Features (Future)

1. **Warning Propagation** - Typst warnings could populate `RenderResult.warnings`
2. **Error Context** - Add operation context (which template, filter, etc.)
3. **Source Code Context** - Show code snippets in `fmt_pretty()` output

### 📋 Phase 3: Polish (Future)

1. **Source Mapping** - Implement `@origin:` comment anchor system to map Typst errors back to Markdown
2. **JSON Output** - Add `--json` mode for error output
3. **Error Documentation** - Create error code registry with explanations

---

## Usage

See implementation files for complete API documentation and examples:
- `quillmark-core/src/error.rs` - Core error types
- `quillmark-typst/src/error_mapping.rs` - Typst error mapping
- `quillmark-typst/src/compile.rs` - Compilation error handling

---

## Best Practices

### For Backend Implementors

1. **Always return structured errors** - Convert native errors to `Diagnostic` objects
2. **Provide helpful hints** - Match common error patterns and suggest concrete fixes
3. **Use proper error codes** - Format: `backend::error_type` (e.g., `typst::unknown_variable`)
4. **Test error paths** - Verify error messages are helpful and locations are accurate

### For Application Developers

1. **Use `print_errors()` for CLI** - Consistent formatting across error types
2. **Serialize for tooling** - All diagnostic types implement `serde::Serialize`
3. **Don't ignore warnings** - Check `RenderResult.warnings` and log or display to users

---

## Migration Notes

### From Pre-Phase 1 Code

1. **Typst errors are now structured** - Access diagnostics array instead of parsing strings
2. **No more panics on compilation** - Handle `Result` types explicitly
3. **MiniJinja column info now accurate** - No code changes needed, just better output

---

## Future Improvements

### Considered for Phase 2
- Enhanced Diagnostics (source code snippets, color coding, multi-line error spans)
- Warning System (propagate Typst warnings, custom warning types)
- Error Context (operation stack traces, template instantiation history)

### Considered for Phase 3
- Advanced Source Mapping (`@origin:` comment injection, source map generation)
- Tooling Integration (LSP server support, IDE error highlighting)
- Error Documentation (error code registry, online documentation links)


