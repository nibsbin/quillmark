# Error Handling System

> **Implementation**: `crates/core/src/error.rs`

## Types

**`Severity`**: `Error` | `Warning` | `Note`

**`Location`**: file name, line (1-indexed), column (1-indexed)

**`Diagnostic`**: severity, optional error code, message, primary location, optional hint, source error chain (omitted from serialization when empty)

**`ParseError`**: parsing-stage error enum — input too large, YAML errors (with and without location), invalid structure; converts to `Diagnostic` via `to_diagnostic()`

**`RenderError`**: main rendering error enum with variants:
- `EngineCreation` — failed to create engine
- `InvalidFrontmatter` — malformed YAML frontmatter (also wraps `ParseError`)
- `CompilationFailed` — backend compilation failed; carries `Vec<Diagnostic>`
- `FormatNotSupported` — requested output format not supported
- `UnsupportedBackend` — backend not registered
- `ValidationFailed` — field coercion/schema validation failure
- `QuillConfig` — quill configuration error

**`RenderResult`**: successful result carrying artifacts, output format, and non-fatal `Vec<Diagnostic>` warnings

## Bindings Error Delegation

Python and WASM bindings delegate to core types:

- **Python**: `PyDiagnostic` wraps `Diagnostic`. `RenderError` is mapped to typed Python exceptions: `CompilationError` (carries a `diagnostics` list), `ParseError` (frontmatter errors), and `QuillmarkError` (all other variants) — each with an attached `diagnostic` attribute. Base hierarchy: `QuillmarkError → PyException`.
- **WASM**: `WasmError` is a flat `{ message, diagnostics: Vec<Diagnostic> }`. The thrown JS `Error` has `.message` and a `.diagnostics` array attached (always non-empty — length 1 for single-diagnostic errors, length N for compilation failures). Same shape regardless of underlying variant; consumers read `err.diagnostics[0]` for the primary diagnostic.

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
