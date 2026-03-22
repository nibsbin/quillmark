# Python Package Design for Quillmark

Status: **Implemented** (2026-03-22)  
Package: `quillmark` (PyPI) — Python 3.10+

## Surface
- `Quillmark()`
  - `register_quill(quill)`  
  - `workflow(quill_ref | Quill | ParsedDocument)` → `Workflow`
  - `registered_backends()` / `registered_quills()` / `get_quill(name_or_ref)`
- `Workflow`
  - `render(parsed, format: OutputFormat | None = None)` → `RenderResult`
  - `dry_run(parsed)` (coercion + schema validation only)
  - `backend_id`, `supported_formats`, `quill_ref` properties
  - Dynamic assets/fonts: `add_asset(s)`, `add_font(s)`, `clear_assets/fonts`, name getters
- `Quill`
  - `from_path(path)` static constructor
  - Props: `name`, `backend`, `metadata`, `schema`, `defaults`, `examples`, `example`, `plate`
- `ParsedDocument`
  - `from_markdown(markdown)` static
  - `body()`, `get_field(key)`, `fields()`, `quill_ref()`
- `RenderResult`
  - `artifacts`, `warnings`, `output_format`
- `Artifact`
  - `bytes`, `output_format`, `mime_type`, `save(path)`

## Enums / Formats
- `OutputFormat`: `PDF`, `SVG`, `PNG`, `TXT`
- `Severity`: `ERROR`, `WARNING`, `NOTE`

## Errors
- Exceptions mirror core `RenderError`: `ParseError`, `TemplateError`, `CompilationError`, base `QuillmarkError`.
- Payloads carry `SerializableDiagnostic` (code/message/location/hint/source chain).

## Notes
- Backend support list derived from registered backends (Typst: pdf/svg/png/txt; AcroForm: pdf).
- The exact JSON returned by `Workflow::compile_data()` is exposed via `render().warnings` diagnostics and `output_data` flow in CLI parity.

---

## Distribution & Packaging

**PyPI Distribution:**
- Binary wheels for major platforms (Linux, macOS, Windows)
- Multiple Python versions (3.10+)
- Source distribution as fallback

**Installation:**
```bash
pip install quillmark
uv pip install quillmark
```
