# Error Handling System

> **Implementation**: `crates/core/src/error.rs`

## Types

**`Severity`**: `Error` | `Warning` | `Note`

**`Location`**: file name, line (1-indexed), column (1-indexed)

**`Diagnostic`**: severity, optional error code, message, primary location, optional hint, source error chain (skipped from serialization)

**`SerializableDiagnostic`**: flattened `Diagnostic` for Python and WASM FFI — replaces the non-serializable `source` field with a `source_chain: Vec<String>`

**`ParseError`**: parsing-stage error enum — input too large, YAML errors (with and without location), JSON conversion, invalid structure, missing CARD directive; converts to `Diagnostic` via `to_diagnostic()`

**`RenderError`**: main rendering error enum with variants:
- `EngineCreation` — failed to create engine
- `InvalidFrontmatter` — malformed YAML frontmatter (also wraps `ParseError`)
- `CompilationFailed` — backend compilation failed; carries `Vec<Diagnostic>`
- `FormatNotSupported` — requested output format not supported
- `UnsupportedBackend` — backend not registered
- `ValidationFailed` — field coercion/schema validation failure
- `QuillConfig` — quill configuration error
- `NoBackend` — quill has no backend attached

**`RenderResult`**: successful result carrying artifacts, output format, and non-fatal `Vec<Diagnostic>` warnings

## Bindings Error Delegation

Python and WASM bindings delegate to core types:

- **Python**: `PyDiagnostic` wraps `SerializableDiagnostic`. `RenderError` is mapped to typed Python exceptions: `CompilationError` (carries a `diagnostics` list), `ParseError` (frontmatter errors), and `QuillmarkError` (all other variants) — each with an attached `diagnostic` attribute. Base hierarchy: `QuillmarkError → PyException`.
- **WASM**: `WasmError` wraps `SerializableDiagnostic` as either `Diagnostic` (single) or `MultipleDiagnostics` (compilation errors), serialized to JSON via `serde_wasm_bindgen`. Also handles `ParseError` conversions directly.

## Backend Error Mapping

### Typst

Typst diagnostics mapped via `map_typst_errors()`:
- Severity levels mapped (Error/Warning)
- Spans resolved to file/line/column
- Error codes: `"typst::<error_type>"`

See `crates/backends/typst/src/error_mapping.rs`.

## Error Presentation

**Pretty printing** (`Diagnostic::fmt_pretty()`):
```
[ERROR] Undefined variable (E001)
  --> template.typ:10:5
  hint: Check variable spelling
```

**Extended printing** (`Diagnostic::fmt_pretty_with_source()`): appends each cause in the source chain as `cause N: <message>`.

**Consolidated printing**: `print_errors()` handles all `RenderError` variants.

**Machine-readable**: all diagnostic types implement `serde::Serialize`.
