# Error Handling System

> **Implementation**: `quillmark-core/src/error.rs`

## Types

**`Severity`**: `Error` | `Warning` | `Note`

**`Location`**: file name, line (1-indexed), column (1-indexed)

**`Diagnostic`**: severity, optional error code, message, primary location, optional hint, source error chain

**`RenderError`**: main error enum — engine creation, invalid frontmatter, template rendering, backend compilation (may contain multiple diagnostics), format/backend support, resource collision, size limits, schema validation, invalid schema, quill configuration

**`SerializableDiagnostic`**: flattened `Diagnostic` for Python and WASM FFI boundaries

## Bindings Error Delegation

Python and WASM bindings delegate to core types:

- **Python**: `PyDiagnostic` wraps `SerializableDiagnostic`, converting `RenderError` to Python exceptions with attached diagnostic payloads
- **WASM**: `WasmError` wraps `SerializableDiagnostic` (single or multiple), serialized to JSON via `serde_wasm_bindgen`

## Backend Error Mapping

### Typst

Typst diagnostics mapped via `map_typst_errors()`:
- Severity levels mapped (Error/Warning)
- Spans resolved to file/line/column
- Error codes: `"typst::<error_type>"`

See `backends/quillmark-typst/src/error_mapping.rs`.

## Error Presentation

**Pretty printing** (`Diagnostic::fmt_pretty()`):
```
[ERROR] Undefined variable (E001) at template.typ:10:5
  hint: Check variable spelling
```

**Consolidated printing**: `print_errors()` handles all `RenderError` variants.

**Machine-readable**: all diagnostic types implement `serde::Serialize`.
